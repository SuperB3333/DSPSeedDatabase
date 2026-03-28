import psycopg2, io, csv, threading

from misc import distance, StarType, SpectrType, veins

import dsp_generator # Rust Extention

# --- CONFIGURATION ---
DB_HOST = "localhost"
DB_NAME = "dsp"
DB_USER = "postgres"
DB_PASS = "rootpassword"

START_SEED = 0
TOTAL_SEEDS_TO_GENERATE = 100
COMMIT_BATCH_SIZE = 50  # Commit to DB every X galaxies


from profiler import prof


def get_db_connection():
    conn = psycopg2.connect(host=DB_HOST, database=DB_NAME, user=DB_USER, password=DB_PASS)
    return conn, conn.cursor()

def create_schema(cursor):
    c = lambda *_: "" # Return an empty string. Used for comments in the long sql queries
    print("--- Recreating Database Schema ---")

    # 1. Clean up old tables
    cursor.execute("DROP TABLE IF EXISTS planets;")
    cursor.execute("DROP TABLE IF EXISTS stars;")


    # 3. Create star table
    cursor.execute(f"""
        CREATE TABLE stars (
            id INT UNIQUE PRIMARY KEY,
            seed INT,
            
            start_dist FLOAT,
            star_index INT,
            luminosity FLOAT,
            dyson_radius INT,
            type INT, {c("Needs an enum to get the star type")}
            spectr INT, {c("same here")}
            
            {c('\n'.join([f"dist_{spectr} FLOAT," for spectr in SpectrType.keys()]) + "#TODO implement this")}
            {',\n'.join([f"ore_{vein} INT" for vein in veins])}  {c("Add an int value for each of the ores")}
        );
    """)
    cursor.execute(f"""
        CREATE TABLE planets (
            star_id INT,
            
            index INT,
            orbiting INT, {c("Index of gas giant, -1 if orbit around sun")}
            water_item INT, {c("Water Item ID")}
            gas_giant BOOL,
            sun_distance FLOAT,
            inside_ds BOOL, {c("Whether the planet lies inside the Dyson sphere or not")}
            satellites INT, {c("Amount od satellites")}
            tidal_lock BOOL,
            temperature FLOAT,
            theme_id INT,
            
            gas_h FLOAT, {c("Hydrogen")}
            gas_d FLOAT, {c("Deuterium")}
            gas_i FLOAT, {c("Fireice")}
            
            {"\n".join([f"estimate_{ore} INT," for ore in veins])}            
            {"\n".join([f"min_{ore} INT," for ore in veins])}            
            {",\n".join([f"max_{ore} INT" for ore in veins])}            
        );
    """)

    # 5. Create Indexes for Speed
    print("Creating Indexes... (Can be optimized for specific patterns)")
    def index(table: str, val: str) -> None:
        cursor.execute(f"CREATE INDEX idx_{table}_{val} ON {table}({val});")

    index("stars", "id")
    index("planets", "star_id")

    index("stars", "seed")
    index("stars", "star_index")
    index("stars", "dyson_radius")
    index("stars", "luminosity")
    index("stars", "type")
    index("stars", "spectr")

    index("planets", "gas_giant")
    index("planets", "temperature")

class BulkInserter:
    def __init__(self, table: str, columns: list[str]):
        self.buf = io.StringIO()
        self.writer = csv.writer(self.buf)
        self.cols = columns
        self.table = table
        self.commit_at, self.auto_cur, self.cur_pagesize = 0, None, 0
        self.commit_thread = None
        self.buf_mutex = threading.Lock()

    @prof.register
    def add_row(self, values):
        rows_formatted = ['t' if val is True else ('f' if val is False else ('n' if val is None else str(val))) for val in values]  # no arbitrary strings, so a short null string just saves resources
        self.cur_pagesize += 1
        with self.buf_mutex:
            self.writer.writerow(rows_formatted)
        if 0 < self.commit_at <= self.cur_pagesize:
            self.commit()

    def autocommit(self, cur: psycopg2._psycopg.cursor, max_page=COMMIT_BATCH_SIZE):
        self.commit_at = max_page
        self.auto_cur = cur

    def commit(self):
        self.commit_thread = threading.Thread(daemon=True, target=self._thread_commit)
        self.commit_thread.start()

    def _thread_commit(self):
        self.cur_pagesize = 0
        local_conn, local_cur = get_db_connection()
        try:
            sql = f"COPY {self.table} ({", ".join(self.cols)}) FROM STDIN WITH (FORMAT csv, NULL 'n')"
            with self.buf_mutex:
                self.buf.seek(0)
                local_cur.copy_expert(sql, self.buf)
                self.buf.truncate(0)
                self.buf.seek(0)
            local_conn.commit()
        finally:
            local_cur.close(); local_conn.close()

prows = [
    "star_id",
    "index",
    "water_item",
    "gas_giant",
    "sun_distance",
    "inside_ds",
    "satellites",
    "temperature",
    "theme_id",
    "gas_h", "gas_d", "gas_i",
    "tidal_lock"
]
[prows.extend(["min_" + x, "max_" + x, "estimate_" + x]) for x in veins]

planet_inserter = BulkInserter("planets", prows)
srows = ["id", "seed", "start_dist", "star_index", "luminosity", "dyson_radius", "type", "spectr"] + ["ore_" + x for x in veins]
star_inserter = BulkInserter("stars", srows)

VEIN_TITLES = [ore.title() for ore in veins]

@prof.register # Profiler
def process_galaxy(seed: int, star_count=64, resource_mult=1):

    with prof.inspect("generate"):
        galaxy_data = dsp_generator.generate(seed, star_count, resource_mult) # Rust module

    for solar_system in galaxy_data:
        star = solar_system["star"]

        star_id = seed * 100 + star["index"]

        vals = [star_id, seed, distance(star["position"]), star["index"], star["luminosity"], star["dysonRadius"], StarType.get(star["type"]), SpectrType.get(star["spectr"])]

        avg_veins = solar_system["avg_veins"]
        vals += [int(avg_veins.get(title)) for title in VEIN_TITLES]

        star_inserter.add_row(vals)
        del vals # Free up memory, the same variable name is used at multiple places

        for planet in solar_system["planets"]:
            gas_dict = dict(planet["gases"])

            # for gas_dict.get it says it only accepts str as keys, but int works as well
            # noinspection PyTypeChecker
            p_vals = [
                star_id,
                planet["index"],
                planet["theme"]["waterItemId"],
                planet["type"] == "Gas",
                planet["orbitRadius"],
                planet["orbitRadius"] * 40_000 < star["dysonRadius"],
                -1,  # todo satellites, implement
                planet["theme"]["temperature"],
                planet["theme"]["id"],
                gas_dict.get(1120, 0.0),
                gas_dict.get(1121, 0.0),
                gas_dict.get(1011, 0.0),
                planet["rotationPeriod"] == planet["orbitalPeriod"]
            ]
            if planet["type"] == "Gas":
                p_vals.extend([-1] * 42) # 3 for each ore (14 ores)
                planet_inserter.add_row(p_vals)
                continue
            planet_veins = {v_est["veinType"].lower(): {k: v for k, v in v_est.items() if k != "veinType"} for v_est in planet["veins"]} # Expand into a dictionary
            for ore in veins:
                dat = planet_veins.get(ore, None)
                if dat is None: p_vals.extend([-1, -1, -1])
                else:
                    p_vals.append(int(dat["minGroup"] * dat["minPatch"] * dat["minAmount"]))
                    p_vals.append(int(dat["maxGroup"] * dat["maxPatch"] * dat["maxAmount"]))
                    p_vals.append(int(((dat["minGroup"] + dat["maxGroup"]) *
                                       (dat["minPatch"] + dat["maxPatch"]) *
                                       (dat["minAmount"] + dat["maxAmount"])) // 8))


            planet_inserter.add_row(p_vals)


def main():
    conn, cur = get_db_connection()

    try:
        # Create Schema
        create_schema(cur)
        conn.commit()  # Commit schema changes

        print(f"Starting generation of {TOTAL_SEEDS_TO_GENERATE} seeds...")

        planet_inserter.autocommit(cur)
        star_inserter.autocommit(cur)

        for i in range(TOTAL_SEEDS_TO_GENERATE):
            current_seed = START_SEED + i

            # Process one galaxy
            process_galaxy(current_seed)

            # Batch Commit
            if (i + 1) % COMMIT_BATCH_SIZE == 0:
                conn.commit()
                print(f"Committed {i + 1} seeds.")

        # Final Commit
        conn.commit()
        print("Generation Complete.")
        print(f"{'-' * 100}\nProfiling Results:\n")
        prof.print_results()
    except Exception:
        print(f"An error occurred!")
        conn.rollback()
        raise
    finally:
        planet_inserter.commit()
        star_inserter.commit()
        cur.close()
        conn.close()


if __name__ == "__main__":
    main()
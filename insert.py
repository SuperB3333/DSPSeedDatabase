import psycopg2, time

from misc import distance, StarType, SpectrType, veins

import dsp_generator # Rust Extention

# --- CONFIGURATION ---
DB_HOST = "localhost"
DB_NAME = "dsp"
DB_USER = "postgres"
DB_PASS = "rootpassword"

START_SEED = 0
TOTAL_SEEDS_TO_GENERATE = 1000
COMMIT_BATCH_SIZE = 100  # Commit to DB every X galaxies



def get_db_connection():
    conn = psycopg2.connect(host=DB_HOST, database=DB_NAME, user=DB_USER, password=DB_PASS)
    return conn


def create_schema(cursor):
    c = lambda *_: "" # Return an empty string. Used for comments in the long sql queries
    print("--- Recreating Database Schema ---")

    # 1. Clean up old tables
    cursor.execute("DROP TABLE IF EXISTS planets;")
    cursor.execute("DROP TABLE IF EXISTS stars;")


    # 3. Create star table
    cursor.execute(f"""
        CREATE TABLE stars (
            id SERIAL UNIQUE PRIMARY KEY,
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
            star_id SERIAL,
            
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
    index("planets", "gas_d")



def process_galaxy(cursor, seed, star_count=64, resource_mult=1):
    """
        Generates one galaxy and inserts it, its stars, and its planets using 3 SQL queries.
    """

    def dict_to_sql(table, values, get_id=False):
        cols = ", ".join(values.keys())
        vals = tuple(values.values())
        placeholders = ", ".join(["%s"] * len(vals))
        query = f"INSERT INTO {table} ({cols}) VALUES ({placeholders}) {"RETURNING id" if get_id else ""}"
        cursor.execute(query, vals)
        if get_id: return cursor.fetchone()[0]

    # 1. Generate Data (Rust)
    # The result is a list of star dictionaries
    galaxy_data = dsp_generator.generate(seed, star_count, resource_mult)




    for solar_system in galaxy_data:
        star = solar_system["star"]
        vals = {
            "seed": seed, #todo implement distance from spectr

            "start_dist": distance(star["position"]),
            "star_index": star["index"],
            "luminosity": star["luminosity"],
            "dyson_radius": star["dysonRadius"],
            "type": StarType.get(star["type"]),
            "spectr": SpectrType.get(star["spectr"])
        }
        for ore in veins:
            vals[f"ore_{ore}"] = solar_system["avg_veins"][ore.title()]

        star_id = dict_to_sql("stars", vals, get_id=True)

        for planet in solar_system["planets"]:
            vals = {
                "star_id": star_id,
                "index": planet["index"],
                "water_item": planet["theme"]["waterItemId"],
                "gas_giant": planet["type"] == "Gas",
                "sun_distance": planet["orbitRadius"],
                "inside_ds": planet["orbitRadius"] * 40_000 < star["dysonRadius"],
                "satellites": -1, #TODO implement
                "temperature": planet["theme"]["temperature"],
                "theme_id": planet["theme"]["id"],
                "gas_h": -1,
                "gas_d": -1, #todo implement these
                "gas_i": -1

            }
            planet_veins = {v_est["veinType"].lower(): {k: v for k, v in v_est.items() if k != "veinType"} for v_est in planet["veins"]} # Expand intp a dictoinaty
            for ore in veins:

                dat = planet_veins.get(ore, None)

                if planet["type"] == "Gas" or dat is None:
                    vals[f"min_{ore}"], vals[f"max_{ore}"], vals[f"estimate_{ore}"] = -1, -1, -1
                    break
                vals[f"min_{ore}"] = dat["minGroup"] * dat["minPatch"] * dat["minAmount"]
                vals[f"max_{ore}"] = dat["maxGroup"] * dat["maxPatch"] * dat["maxAmount"]
                vals[f"estimate_{ore}"] = ((dat["minGroup"] + dat["maxGroup"]) *
                    (dat["minPatch"] + dat["maxPatch"]) *
                    (dat["minAmount"] + dat["maxAmount"])) / 8

            dict_to_sql("planets", vals) #todo implement tidal lock column

def main():
    conn = get_db_connection()
    cur = conn.cursor()

    try:
        # Create Schema
        create_schema(cur)
        conn.commit()  # Commit schema changes

        print(f"Starting generation of {TOTAL_SEEDS_TO_GENERATE} seeds...")
        start_time = time.time()

        for i in range(TOTAL_SEEDS_TO_GENERATE):
            current_seed = START_SEED + i

            # Process one galaxy
            process_galaxy(cur, current_seed)

            # Batch Commit
            if (i + 1) % COMMIT_BATCH_SIZE == 0:
                conn.commit()
                elapsed = time.time() - start_time
                rate = (i + 1) / elapsed
                print(f"Committed {i + 1} seeds. Rate: {rate:.2f} galaxies/sec")

        # Final Commit
        conn.commit()
        print("Generation Complete.")

    except Exception:
        print(f"An error occurred!")
        conn.rollback()
        raise
    finally:
        cur.close()
        conn.close()


if __name__ == "__main__":
    main()
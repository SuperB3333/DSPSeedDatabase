import psycopg2

from misc import SpectrType, veins

# --- CONFIGURATION ---
DB_HOST = "localhost"
DB_NAME = "dsp"
DB_USER = "postgres"
DB_PASS = "rootpassword"


def create_schema():
    conn = psycopg2.connect(host=DB_HOST, database=DB_NAME, user=DB_USER, password=DB_PASS)
    cursor = conn.cursor()

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

    conn.commit()

if __name__ == "__main__":
    create_schema()
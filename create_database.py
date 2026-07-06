import psycopg2
import argparse

from misc import veins

# --- CONFIGURATION ---
DB_HOST = "localhost"
DB_NAME = "dsp"
DB_USER = "postgres"
DB_PASS = "rootpassword"


def create_schema():
    conn = psycopg2.connect(host=DB_HOST, database=DB_NAME, user=DB_USER, password=DB_PASS)
    cursor = conn.cursor()

    print("--- Recreating Database Schema ---")

    # Clean up old tables
    cursor.execute("DROP TABLE IF EXISTS planets;")
    cursor.execute("DROP TABLE IF EXISTS stars;")

    ore_cols = ",\n".join([f"ore_{ore} INT" for ore in veins])

    cursor.execute(f"""
        CREATE UNLOGGED TABLE stars (
            id INT PRIMARY KEY,
            seed INT,
            dyson_radius INT,
            {ore_cols},
            start_dist REAL,
            luminosity REAL,
            star_index SMALLINT,
            type SMALLINT,
            spectr SMALLINT
        );
    """)

    vein_cols = ",\n".join(
        [f"estimate_{ore} INT" for ore in veins] +
        [f"min_{ore} INT" for ore in veins] +
        [f"max_{ore} INT" for ore in veins]
    )

    cursor.execute(f"""
        CREATE UNLOGGED TABLE planets (
            star_id INT,
            {vein_cols},
            sun_distance REAL,
            temperature REAL,
            gas_h REAL,
            gas_d REAL,
            gas_i REAL,
            index SMALLINT,
            orbiting SMALLINT,
            water_item SMALLINT,
            satellites SMALLINT,
            theme_id SMALLINT,
            gas_giant BOOL,
            inside_ds BOOL,
            tidal_lock BOOL,
            UNIQUE(star_id, index)
        );
    """)

    conn.commit()
    cursor.close()
    conn.close()

    print("Schema created. Tables are UNLOGGED for faster writes; "
          "run ALTER TABLE stars SET LOGGED; ALTER TABLE planets SET LOGGED; "
          "if durability is desired.")


def create_indexes():
    conn = psycopg2.connect(host=DB_HOST, database=DB_NAME, user=DB_USER, password=DB_PASS)
    cursor = conn.cursor()

    print("Creating Indexes...")
    def index(table, val):
        cursor.execute(f"CREATE INDEX idx_{table}_{val} ON {table}({val});")

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
    cursor.close()
    conn.close()

    print("Indexes created.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--indexes", action="store_true", help="Create indexes only")
    args = parser.parse_args()

    if args.indexes:
        create_indexes()
    else:
        create_schema()

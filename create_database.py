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
    cursor.execute("DROP TABLE IF EXISTS themes;")

    cursor.execute("""
        CREATE TABLE themes (
            id SMALLINT PRIMARY KEY,
            name TEXT,
            temperature REAL,
            ocean_type SMALLINT
        );
    """)
    cursor.execute("""
        INSERT INTO themes (id, name, temperature, ocean_type) VALUES
        (1, 'Ocean 1', 0.0, 1000),
        (2, 'Gas 1', 2.0, 0),
        (3, 'Gas 2', 1.0, 0),
        (4, 'Gas 3', -1.0, 0),
        (5, 'Gas 4', -2.0, 0),
        (6, 'Desert 1', 2.0, 0),
        (7, 'Desert 2', -1.0, 0),
        (8, 'Ocean 2', 0.0, 1000),
        (9, 'Lava 1', 5.0, -1),
        (10, 'Ice 1', -5.0, 1000),
        (11, 'Desert 3', -2.0, 0),
        (12, 'Desert 4', 1.0, 0),
        (13, 'Volcanic 1', 4.0, 1116),
        (14, 'Ocean 3', 0.0, 1000),
        (15, 'Ocean 4', 0.0, 1000),
        (16, 'Ocean 5', 0.0, 1000),
        (17, 'Desert 5', 1.0, 0),
        (18, 'Ocean 6', 0.0, 1000),
        (19, 'Desert 6', 1.0, 0),
        (20, 'Desert 7', -2.0, -2),
        (21, 'Gas 5', 1.0, 0),
        (22, 'Desert 8', 0.0, 1000),
        (23, 'Desert 9', 0.08, 0),
        (24, 'Desert 10', -4.0, 0),
        (25, 'Desert 11', 0.0, 1000);
    """)

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
            gas_h REAL,
            gas_d REAL,
            gas_i REAL,
            index SMALLINT,
            orbiting SMALLINT,
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

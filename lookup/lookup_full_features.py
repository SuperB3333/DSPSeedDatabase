from rules import *

import psycopg2

# DB Config
DB_CONFIG = {
    "host": "localhost",
    "database": "dsp",
    "user": "postgres",
    "password": "rootpassword"
}
def get_db_connection():
    conn = psycopg2.connect(**DB_CONFIG)
    return conn, conn.cursor()

def find_seeds_by_rule(rule_root: GenericRule):
    """
    Takes a Python Rule Object, compiles it to SQL, and finds matching Seeds.
    """
    # 1. Compile Rule to SQL WHERE clause
    where_clause, params = rule_root.to_sql()

    # 2. Construct the full query
    # We join galaxies to stars, apply the filter, and return unique Seeds.
    query = f"""
        SELECT DISTINCT g.seed
        FROM galaxies g
        JOIN stars s ON g.id = s.galaxy_id
        WHERE {where_clause}
        LIMIT 100;
    """

    # 3. Execute
    try:
        conn, cur = get_db_connection()

        print(f"DEBUG SQL: {query}")
        print(f"DEBUG PARAMS: {params}")

        cur.execute(query, params)
        results = cur.fetchall()

        conn.close()
        return [row[0] for row in results]

    except Exception as e:
        print(f"Database error: {e}")
        return []


# --- Example Usage ---

if __name__ == "__main__":
    # 1. Define the complex rule
    rule_tree = StarAmountRule(AndRule([
        StarSpectrRule("O"),
        StarTypeRule("GiantStar")
    ]), 2 , SQLOperator.gte)

    # 2. Compile
    full_query, params = rule_tree.to_sql()

    print("--- Generated SQL ---")
    print(full_query)
    print("Params:", params)
    conn, cur = get_db_connection()
    cur.execute(full_query, params)
    res = cur.fetchall()
    print(len(res))
    print(res)
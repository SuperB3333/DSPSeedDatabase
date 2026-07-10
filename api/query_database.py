import threading, psycopg2, os



def _new_connection():
    return psycopg2.connect(database=os.getenv("PG_DBNAME"), user=os.getenv("PG_USER"), password=os.getenv("PG_PASS"), host=os.getenv("PG_NETLOC"), port=os.getenv("PG_PORT"))

_client = None
def _get_client():
    global _client
    if _client is not None: return _client
    _client = _new_connection()
    return _client

def query_database(query: tuple[str, list], qid: str) -> None:
    threading.Thread(target=_query_database, args=(query + "\nLIMIT 100", qid), name=f"Database query {qid}", daemon=True)
def _query_database(query: tuple[str, list], qid: str) -> None:
    global glob_return_dict
    cur = _get_client().cursor()
    cur.execute(*query)
    res = cur.fetchall()

    glob_return_dict[qid] = map(lambda x: x[0], res)
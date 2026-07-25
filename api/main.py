from fastapi import FastAPI
from hashlib import sha256

from query_database import query_database

import time

app = FastAPI()

glob_return_dict = {}

@app.post("/start_query")
def start_query(query: str, params: list):
    h = sha256(query.encode())
    h.update(bytes(int(time.time())))
    qid = h.hexdigest()
    glob_return_dict[qid] = None

    query_database((query, params), qid)

    return qid

@app.post("/query_ready")
def query_status(qid: str):
    return "true" if glob_return_dict[qid] is not None else "false"
@app.post("/get_result")
def query_result(qid: str):
    return list(glob_return_dict[qid])

# server.py
import asyncio, json, psycopg2
from websockets import serve, server

from parse_rule import parse as parse_rule
from dsp_generator import generate as generate_galaxy

DB_CONFIG = {
    "host": "localhost",
    "database": "dsp",
    "user": "postgres",
    "password": "rootpassword"
}

def create_galaxy(game):
    return generate_galaxy(game["seed"], 8, 8) #todo input star_count and resource_multiplier as well


def find_stars(options):
    sql, params = parse_rule(options)
    print(f"SQL Code:\n{sql}\n{'-' * 100}\n{params}")
    conn = psycopg2.connect(**DB_CONFIG)
    cur = conn.cursor()
    cur.execute(sql, params)
    results = cur.fetchall()
    print(f"\n\n{'-' * 100}\nResults:\n{results}")
    return [item[0] for item in results] # Results are tuples of one item each


async def handle_find(ws: server, payload):
    # Flatten payload
    payload |= payload.pop("game")
    payload["start"], payload["end"] = payload.pop("range")
    [payload.pop(i, None) for i in ["type", "concurrency", "autosave"]]

    results = find_stars(payload)

    for seed in results:
        await ws.send(json.dumps({
            "type": "Result",
            "seed": seed
        }))

    # finally send Done
    await ws.send(json.dumps({
        "type": "Done",
        "start": payload["start"],
        "end": payload["end"]
    }))


async def handler(ws: server):
    async for message in ws:
        try:
            msg = json.loads(message)
        except json.JSONDecodeError:
            continue

        t = msg.get("type")
        match t:
            case "Stop":
                await ws.close()
                return
            case "Generate":
                galaxy = create_galaxy(msg["game"])
                await ws.send(json.dumps(galaxy))
            case "Find":
                await handle_find(ws, msg)


async def main():
    async with serve(handler, "127.0.0.1", 62879):
        print("WebSocket server started on ws://127.0.0.1:62879")
        await asyncio.Future()  # run forever


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass

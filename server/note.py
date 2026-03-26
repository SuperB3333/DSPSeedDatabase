d = {'type': 'Find', 'game': {'resourceMultiplier': 1, 'starCount': 64}, 'range': [0, 100000000], 'rule': {'type': 'Composite', 'condition': {'type': 'Gte', 'value': 1}, 'rule': {'type': 'BirthDistance', 'condition': {'type': 'Lte', 'value': 1}}}, 'concurrency': 12, 'autosave': 5}

from pprint import pprint as pprint
import json
d |= d.pop("game") # Flatten game
d["start"], d["end"] = d.pop("range") # Flatten range out
d.pop("type")

print(json.dumps(d.get("rule"), indent=4))
import math


def distance(pos1, pos2 = (0, 0, 0)) -> float:
    return math.sqrt((pos1[0] - pos2[0]) ** 2 + (pos1[1] - pos2[1]) ** 2 + (pos1[2] - pos2[2]) ** 2)



StarType = {
    "MainSeqStar": 1,
    "GiantStar": 2,
    "WhiteDwarf": 3,
    "NeutronStar": 4,
    "BlackHole": 5
}
SpectrType = {
    "M": -4,
    "K": -3,
    "G": -2,
    "F": -1,
    "A":  0,
    "B":  1,
    "O":  2,
    "X":  3
}


veins = ["iron", "copper", "silicium", "titanium", "stone", "coal", "oil", "fireice", "diamond", "fractal", "crysrub",
         "grat", "bamboo", "mag"]

def no_indent(s: str) -> str:
    return '\n'.join(map(str.lstrip, s.splitlines())).lstrip('\n')
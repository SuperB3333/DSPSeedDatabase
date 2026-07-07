from typing import Tuple, List, Any, Union
from misc import SpectrType, StarType

from enum import Enum

class StrEnum(str, Enum):
    def __str__(self):
        return self.value

# --- Base Classes ---

class GenericRule:
    def to_sql(self, alias="s") -> Tuple[str, List[Any]]:
        raise NotImplementedError

class AmountRule(GenericRule):
    def to_sql(self, alias="s") -> Tuple[str, List[Any]]:
        raise NotImplementedError

class SQLOperator(StrEnum):
    e = "="
    eq = "="
    ne = "!="
    n = "!="
    gte = ">="
    ge = ">="
    lte = "<="
    le = "<="
    gt = ">"
    g = ">"
    lt = "<"
    l = "<"
    st = "<"

    def sql(self, rule: Union[AmountRule, Tuple[str, List[Any]]], amount: Any) -> Tuple[str, List[Any]]:
        if isinstance(rule, AmountRule):
            rule_sql, params = rule.to_sql()
        else:
            rule_sql, params = rule
        return f"({rule_sql} {self.value} %s)", params + [amount]

# --- Global Rules ---
class StarAmountRule(GenericRule):
    def __init__(self, ruleset: Union[GenericRule, Tuple[str, List[Any]]], amount_stars: int, operand: SQLOperator):
        self.rule = ruleset
        self.amount = amount_stars
        self.op = operand
    def to_sql(self, alias='s'):
        if isinstance(self.rule, GenericRule):
            where, params = self.rule.to_sql(alias='s')
        else:
            where, params = self.rule

        query = f"""
            SELECT s.seed
            FROM stars s
            WHERE {where}
            GROUP BY s.seed
            HAVING COUNT(*) {self.op.value} %s
        """
        return query, params + [self.amount]

class TotalAmountRule(AmountRule):
    def __init__(self, planetary_rule: Union[AmountRule, Tuple[str, List[Any]]]):
        self.rule = planetary_rule

    def to_sql(self, alias="s") -> Tuple[str, List[Any]]:
        if isinstance(self.rule, AmountRule):
            rule_sql, params = self.rule.to_sql(alias="p")
        else:
            rule_sql, params = self.rule

        return f"""(
            SELECT COALESCE(SUM({rule_sql}), 0)
            FROM planets p
            WHERE p.star_id = {alias}.id
        )""", params

# --- Amount Rules ---

class StarVeinRule(AmountRule):
    def __init__(self, vein_type: str):
        self.vein_type = vein_type.lower()
    def to_sql(self, alias="s") -> Tuple[str, List[Any]]:
        return f"{alias}.ore_{self.vein_type}", []

class PlanetVeinRule(AmountRule):
    def __init__(self, vein_type: str):
        self.vein_type = vein_type.lower()
    def to_sql(self, alias="p") -> Tuple[str, List[Any]]:
        return f"{alias}.estimate_{self.vein_type}", []

class AvgVeinRule(AmountRule):
    def __init__(self, vein_type: str):
        self.vein_type = vein_type
    def to_sql(self, alias="s") -> Tuple[str, List[Any]]:
        if alias == "s": return StarVeinRule(self.vein_type).to_sql(alias)
        else: return PlanetVeinRule(self.vein_type).to_sql(alias)

class StartDistanceRule(AmountRule):
    def to_sql(self, alias="s") -> Tuple[str, List[Any]]:
        return f"{alias}.start_dist", []

# --- Property Rules ---

class BirthRule(GenericRule):
    def to_sql(self, alias='s') -> Tuple[str, List[Any]]:
        if alias == 's': return "s.star_index = %s", [0]
        elif alias == 'p': return "p.theme_id = %s", [1]
        return "FALSE", []

class ThemeRule(GenericRule):
    def __init__(self, target_ids: List[int]):
        self.ids = target_ids
    def to_sql(self, alias="p") -> Tuple[str, List[Any]]:
        if not self.ids: return "FALSE", []
        placeholders = ", ".join(["%s"] * len(self.ids))
        return f"{alias}.theme_id IN ({placeholders})", self.ids

# --- Logic Rules ---

class NotRule(GenericRule):
    def __init__(self, rule: Union[GenericRule, Tuple[str, List[Any]]]):
        self.rule = rule
    def to_sql(self, alias="s"):
        if isinstance(self.rule, GenericRule): sql, params = self.rule.to_sql(alias)
        else: sql, params = self.rule
        return f"NOT ({sql})", params

class AndRule(GenericRule):
    def __init__(self, rules: List[Union[GenericRule, Tuple[str, List[Any]]]]):
        self.rules = rules
    def to_sql(self, alias="s"):
        if not self.rules: return "TRUE", []
        clauses, params = [], []
        for rule in self.rules:
            s, p = rule.to_sql(alias) if isinstance(rule, GenericRule) else rule
            clauses.append(f"({s})")
            params.extend(p)
        return " AND ".join(clauses), params

class OrRule(GenericRule):
    def __init__(self, rules: List[Union[GenericRule, Tuple[str, List[Any]]]]):
        self.rules = rules
    def to_sql(self, alias="s"):
        if not self.rules: return "FALSE", []
        clauses, params = [], []
        for rule in self.rules:
            s, p = rule.to_sql(alias) if isinstance(rule, GenericRule) else rule
            clauses.append(f"({s})")
            params.extend(p)
        return " OR ".join(clauses), params

# --- Star Property Rules ---

class StarLuminosityRule(AmountRule):
    def to_sql(self, alias="s"):
        return f"{alias}.luminosity", []

class DysonRadiusRule(AmountRule):
    def to_sql(self, alias="s"):
        return f"{alias}.dyson_radius", []

class StarTypeRule(GenericRule):
    def __init__(self, star_type: Union[str, int]):
        self.star_type = star_type if isinstance(star_type, int) else StarType.get(star_type)
    def to_sql(self, alias="s") -> Tuple[str, List[Any]]:
        return f"{alias}.type = %s", [self.star_type]

class StarSpectrRule(GenericRule):
    def __init__(self, spectr: Union[str, int]):
        self.spectr = spectr if isinstance(spectr, int) else SpectrType.get(spectr)
    def to_sql(self, alias="s") -> Tuple[str, List[Any]]:
        return f"{alias}.spectr = %s", [self.spectr]

class DistanceToSpectrRule(AmountRule):
    def __init__(self, spectr: str):
        self.spectr = spectr
    def to_sql(self, alias="s") -> Tuple[str, List[Any]]:
        spectr_val = SpectrType.get(self.spectr) if isinstance(self.spectr, str) else self.spectr
        sql = f"""
        (SELECT SQRT(POW(s2.position_x - {alias}.position_x, 2) +
                     POW(s2.position_y - {alias}.position_y, 2) +
                     POW(s2.position_z - {alias}.position_z, 2))
         FROM stars s2
         WHERE s2.galaxy_id = {alias}.galaxy_id
         AND s2.spectr = %s
         AND s2.id != {alias}.id
         ORDER BY (POW(s2.position_x - {alias}.position_x, 2) +
                   POW(s2.position_y - {alias}.position_y, 2) +
                   POW(s2.position_z - {alias}.position_z, 2)) ASC
         LIMIT 1)
        """
        return sql, [spectr_val]

class XDistRule(AmountRule):
    def __init__(self, _all=False):
        self.all = _all
    def to_sql(self, alias="s") -> Tuple[str, List[Any]]:
        return f"{alias}.dist_X", []

# --- Planet Rules ---

class PlanetWaterIdRule(AmountRule):
    def to_sql(self, alias="p"):
        return f"{alias}.water_item", []

class GasGiantRule(GenericRule):
    def __init__(self, ice_giants: Union[bool, None] = None):
        self.ice = ice_giants
    def to_sql(self, alias="p") -> Tuple[str, List[Any]]:
        if self.ice is None: return f"{alias}.gas_giant = %s", [True]
        elif self.ice is False: return f"({alias}.gas_giant = %s AND {alias}.temperature >= %s)", [True, 0.0]
        elif self.ice is True:  return f"({alias}.gas_giant = %s AND {alias}.temperature < %s)",  [True, 0.0]
        else: raise TypeError("self.ice was neither True, False or None")

class GasRateRule(AmountRule):
    def __init__(self, gas_type: Union[str, int]):
        if isinstance(gas_type, int):
            if not gas_type in [1120, 1121, 1011]: raise ValueError(f"Gas Type can only be 1120 (Hydrogen), 1121 (Deuterium) or 1011 (Fireice)! It was {gas_type}")
            self.gas_type = {1120: 'h', 1121: 'd', 1011: 'i'}.get(gas_type)
        elif isinstance(gas_type, str):
            if len(gas_type) == 1: self.gas_type = gas_type.lower()
            else:
                self.gas_type = {
                "Hydrogen": "h",
                "Deuterium": "d",
                "Fireice": "i"
                }.get(gas_type)
        else:
             raise ValueError(f"Invalid gas_type: {gas_type}")
    def to_sql(self, alias="p") -> Tuple[str, List[Any]]:
        return f"{alias}.gas_{self.gas_type}", []

class PlanetSunDistanceRule(AmountRule):
    def to_sql(self, alias="p"):
        return f"{alias}.sun_distance", []

class PlanetInsideDysonRule(GenericRule):
    def to_sql(self, alias="p"):
        return f"{alias}.inside_ds", []

class SatelliteCountRule(AmountRule):
    def to_sql(self, alias="p"):
        sql = f"""
        (SELECT COUNT(*) FROM planets p2
         WHERE p2.star_id = {alias}.star_id
         AND p2.orbiting = {alias}.index)
        """
        return sql, []

class TidalLockRule(GenericRule):
    def to_sql(self, alias="p") -> Tuple[str, List[Any]]:
        return f"{alias}.tidal_lock", []

class PlanetCountRule(AmountRule):
    def __init__(self, planet_rule: GenericRule = None):
        self.planet_rule = planet_rule
    def to_sql(self, alias="s") -> Tuple[str, List[Any]]:
        if self.planet_rule:
            rule_sql, rule_params = self.planet_rule.to_sql(alias="p")
            sql = f"(SELECT COUNT(*) FROM planets p WHERE p.star_id = {alias}.id AND {rule_sql})"
            return sql, rule_params
        else:
            sql = f"(SELECT COUNT(*) FROM planets p WHERE p.star_id = {alias}.id)"
            return sql, []

class HasPlanetRule(GenericRule):
    def __init__(self, planet_rule: Union[GenericRule, Tuple[str, List[Any]]]):
        self.planet_rule = planet_rule
    def to_sql(self, alias="s"):
        if isinstance(self.planet_rule, GenericRule):
            inner_sql, inner_params = self.planet_rule.to_sql(alias="p")
        else:
            inner_sql, inner_params = self.planet_rule
        sql = f"EXISTS (SELECT 1 FROM planets p WHERE p.star_id = {alias}.id AND {inner_sql})"
        return sql, inner_params

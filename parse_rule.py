import json
from rules import *
from typing import Tuple, List, Any


# this is necessary for some reason, but everything works fine
# noinspection PyTypeChecker, PyArgumentList
def parse(data: Union[str, dict]) -> Tuple[str, List[Any]]:
    if isinstance(data, str):
        try:
            data = json.loads(data)
        except json.JSONDecodeError:
            raise TypeError("Input data must be valid json!")

    # Only peel "rule" if it's a wrapper, not if it's a Composite rule
    if isinstance(data, dict) and "rule" in data and "type" not in data:
        data = data["rule"]

    match data["type"]:
        # Logic for galaxies
        case "CompositeAnd":
            parsed = [parse(r) for r in data["rules"]]
            queries, all_params = [], []
            for q, p in parsed:
                queries.append(q)
                all_params.extend(p)
            return "\nINTERSECT\n".join(queries), all_params

        case "CompositeOr":
            parsed = [parse(r) for r in data["rules"]]
            queries, all_params = [], []
            for q, p in parsed:
                queries.append(q)
                all_params.extend(p)
            return "\nUNION\n".join(queries), all_params

        case "Composite": # Star Amount
            op = SQLOperator[data["condition"]["type"].lower()]
            inner_rule_sql, inner_params = parse(data["rule"])
            star_amount_rule = StarAmountRule((inner_rule_sql, inner_params), data["condition"]["value"], op)
            return star_amount_rule.to_sql()

        # Logic for solar systems (conditions on stars)
        case "And":
            rules = [parse(r) for r in data["rules"]]
            return AndRule(rules).to_sql()
        case "Or":
            rules = [parse(r) for r in data["rules"]]
            return OrRule(rules).to_sql()

        # Other Rules
        case "Luminosity":
            op = SQLOperator[data["condition"]["type"].lower()]
            return op.sql(StarLuminosityRule(), data["condition"]["value"])
        case "DysonRadius":
            op = SQLOperator[data["condition"]["type"].lower()]
            return op.sql(DysonRadiusRule(), data["condition"]["value"])
        case "AverageVeinAmount":
            op = SQLOperator[data["condition"]["type"].lower()]
            # Context-sensitive: Star or Planet
            return op.sql(AvgVeinRule(data["vein"]), data["condition"]["value"])
        case "Spectr":
            return StarSpectrRule(data["spectr"][0]).to_sql()
        case "TidalLockCount":
            op = SQLOperator[data["condition"]["type"].lower()]
            # Planet count with tidal lock
            return op.sql(PlanetCountRule(TidalLockRule()), data["condition"]["value"])
        case "OceanType":
            op = SQLOperator.e
            # This is a planet-level property, wrapping in HasPlanetRule
            inner = op.sql(PlanetWaterIdRule(), data["oceanType"])
            return HasPlanetRule(inner).to_sql()
        case "StarType":
            return StarTypeRule(data["starType"][0]).to_sql()
        case "GasCount":
            op = SQLOperator[data["condition"]["type"].lower()]
            return op.sql(PlanetCountRule(GasGiantRule(data.get("ice"))), data["condition"]["value"])
        case "SatelliteCount":
            op = SQLOperator[data["condition"]["type"].lower()]
            return HasPlanetRule(op.sql(SatelliteCountRule(), data["condition"]["value"])).to_sql()
        case "Birth":
            return BirthRule().to_sql()
        case "ThemeId":
            # Multiple IDs, OR-ed together, then check if ANY planet has it.
            return HasPlanetRule(ThemeRule(data["themeIds"])).to_sql()
        case "PlanetCount":
            op = SQLOperator[data["condition"]["type"].lower()]
            if data.get("excludeGiant"):
                inner = NotRule(GasGiantRule())
            else:
                inner = None
            return op.sql(PlanetCountRule(inner), data["condition"]["value"])
        case "BirthDistance":
            op = SQLOperator[data["condition"]["type"].lower()]
            return op.sql(StartDistanceRule(), data["condition"]["value"])
        case "XDistance":
            op = SQLOperator[data["condition"]["type"].lower()]
            return op.sql(XDistRule(), data["condition"]["value"])
        case "SpectrDistance":
            # This rule returns a distance condition.
            dist_op = SQLOperator[data["distanceCondition"]["type"].lower()]
            return dist_op.sql(DistanceToSpectrRule(data["spectr"]), data["distanceCondition"]["value"])
        case "GasRate":
            op = SQLOperator[data["condition"]["type"].lower()]
            # Total amount of gas rate across all planets in the system
            return op.sql(TotalAmountRule(GasRateRule(data["gasType"])), data["condition"]["value"])
        case "PlanetInDysonCount":
            op = SQLOperator[data["condition"]["type"].lower()]
            if data.get("includeGiant"):
                inner = PlanetInsideDysonRule()
            else:
                inner = AndRule([PlanetInsideDysonRule(), NotRule(GasGiantRule())])
            return op.sql(PlanetCountRule(inner), data["condition"]["value"])
        case _:
            raise ValueError(f"Unknown rule type: {data['type']}")



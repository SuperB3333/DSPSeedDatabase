const StarType = {
    "MainSeqStar": 1,
    "GiantStar": 2,
    "WhiteDwarf": 3,
    "NeutronStar": 4,
    "BlackHole": 5
};

const SpectrType = {
    "M": -4,
    "K": -3,
    "G": -2,
    "F": -1,
    "A":  0,
    "B":  1,
    "O":  2,
    "X":  3
};

class GenericRule {
    toSql(alias = "s") {
        throw new Error("NotImplementedError");
    }
}

class AmountRule extends GenericRule {
    toSql(alias = "s") {
        throw new Error("NotImplementedError");
    }
}

const SQLOperator = {
    e: "=",
    eq: "=",
    ne: "!=",
    n: "!=",
    gte: ">=",
    ge: ">=",
    lte: "<=",
    le: "<=",
    gt: ">",
    g: ">",
    lt: "<",
    l: "<",
    st: "<"
};

function getSqlWithOperator(op, rule, amount) {
    let ruleSql, params;
    if (rule instanceof GenericRule) {
        const result = rule.toSql();
        ruleSql = result.sql;
        params = result.params;
    } else if (Array.isArray(rule)) {
        [ruleSql, params] = rule;
    } else if (rule && typeof rule === 'object' && 'sql' in rule) {
        ruleSql = rule.sql;
        params = rule.params;
    } else {
        throw new Error("Invalid rule for Comparison");
    }
    const opValue = SQLOperator[op] || op;
    return { sql: `(${ruleSql} ${opValue} %s)`, params: [...params, amount] };
}

class StarAmountRule extends GenericRule {
    constructor(ruleset, amountStars, operand) {
        super();
        this.rule = ruleset;
        this.amount = amountStars;
        this.op = operand;
    }
    toSql(alias = 's') {
        let where, params;
        if (this.rule instanceof GenericRule) {
            const result = this.rule.toSql('s');
            where = result.sql;
            params = result.params;
        } else {
            [where, params] = this.rule;
        }

        const opValue = SQLOperator[this.op] || this.op;
        const query = `
            SELECT s.seed
            FROM stars s
            WHERE ${where}
            GROUP BY s.seed
            HAVING COUNT(*) ${opValue} %s
        `;
        return { sql: query, params: [...params, this.amount] };
    }
}

class TotalAmountRule extends AmountRule {
    constructor(planetaryRule) {
        super();
        this.rule = planetaryRule;
    }
    toSql(alias = "s") {
        let ruleSql, params;
        if (this.rule instanceof AmountRule) {
            const result = this.rule.toSql("p");
            ruleSql = result.sql;
            params = result.params;
        } else {
            [ruleSql, params] = this.rule;
        }

        const sql = `(
            SELECT COALESCE(SUM(${ruleSql}), 0)
            FROM planets p
            WHERE p.star_id = ${alias}.id
        )`;
        return { sql, params };
    }
}

class StarVeinRule extends AmountRule {
    constructor(veinType) {
        super();
        this.veinType = (veinType || 'Iron').toLowerCase();
    }
    toSql(alias = "s") {
        return { sql: `${alias}.ore_${this.veinType}`, params: [] };
    }
}

class PlanetVeinRule extends AmountRule {
    constructor(veinType) {
        super();
        this.veinType = (veinType || 'Iron').toLowerCase();
    }
    toSql(alias = "p") {
        return { sql: `${alias}.estimate_${this.veinType}`, params: [] };
    }
}

class AvgVeinRule extends AmountRule {
    constructor(veinType) {
        super();
        this.veinType = veinType;
    }
    toSql(alias = "s") {
        if (alias === "s") return new StarVeinRule(this.veinType).toSql(alias);
        else return new PlanetVeinRule(this.veinType).toSql(alias);
    }
}

class StartDistanceRule extends AmountRule {
    toSql(alias = "s") {
        return { sql: `${alias}.start_dist`, params: [] };
    }
}

class BirthRule extends GenericRule {
    toSql(alias = 's') {
        if (alias === 's') return { sql: "s.star_index = %s", params: [0] };
        else if (alias === 'p') return { sql: "p.theme_id = %s", params: [1] };
        return { sql: "FALSE", params: [] };
    }
}

class ThemeRule extends GenericRule {
    constructor(targetIds) {
        super();
        this.ids = targetIds || [];
    }
    toSql(alias = "p") {
        if (!this.ids || this.ids.length === 0) return { sql: "FALSE", params: [] };
        const placeholders = this.ids.map(() => "%s").join(", ");
        return { sql: `${alias}.theme_id IN (${placeholders})`, params: this.ids };
    }
}

class NotRule extends GenericRule {
    constructor(rule) {
        super();
        this.rule = rule;
    }
    toSql(alias = "s") {
        let sql, params;
        if (this.rule instanceof GenericRule) {
            const result = this.rule.toSql(alias);
            sql = result.sql;
            params = result.params;
        } else {
            [sql, params] = this.rule;
        }
        return { sql: `NOT (${sql})`, params };
    }
}

class AndRule extends GenericRule {
    constructor(rules) {
        super();
        this.rules = rules || [];
    }
    toSql(alias = "s") {
        if (!this.rules || this.rules.length === 0) return { sql: "TRUE", params: [] };
        const clauses = [];
        const params = [];
        for (const rule of this.rules) {
            const { sql: s, params: p } = rule instanceof GenericRule ? rule.toSql(alias) : { sql: rule[0], params: rule[1] };
            clauses.push(`(${s})`);
            params.push(...p);
        }
        return { sql: clauses.join(" AND "), params };
    }
}

class OrRule extends GenericRule {
    constructor(rules) {
        super();
        this.rules = rules || [];
    }
    toSql(alias = "s") {
        if (!this.rules || this.rules.length === 0) return { sql: "FALSE", params: [] };
        const clauses = [];
        const params = [];
        for (const rule of this.rules) {
            const { sql: s, params: p } = rule instanceof GenericRule ? rule.toSql(alias) : { sql: rule[0], params: rule[1] };
            clauses.push(`(${s})`);
            params.push(...p);
        }
        return { sql: clauses.join(" OR "), params };
    }
}

class StarLuminosityRule extends AmountRule {
    toSql(alias = "s") {
        return { sql: `${alias}.luminosity`, params: [] };
    }
}

class DysonRadiusRule extends AmountRule {
    toSql(alias = "s") {
        return { sql: `${alias}.dyson_radius`, params: [] };
    }
}

class StarTypeRule extends GenericRule {
    constructor(starType) {
        super();
        this.starType = typeof starType === 'number' ? starType : StarType[starType];
    }
    toSql(alias = "s") {
        return { sql: `${alias}.type = %s`, params: [this.starType] };
    }
}

class StarSpectrRule extends GenericRule {
    constructor(spectr) {
        super();
        this.spectr = typeof spectr === 'number' ? spectr : SpectrType[spectr];
    }
    toSql(alias = "s") {
        return { sql: `${alias}.spectr = %s`, params: [this.spectr] };
    }
}

class DistanceToSpectrRule extends AmountRule {
    constructor(spectr) {
        super();
        this.spectr = spectr;
    }
    toSql(alias = "s") {
        const spectrVal = typeof this.spectr === 'string' ? SpectrType[this.spectr] : this.spectr;
        const sql = `
        (SELECT SQRT(POW(s2.position_x - ${alias}.position_x, 2) +
                     POW(s2.position_y - ${alias}.position_y, 2) +
                     POW(s2.position_z - ${alias}.position_z, 2))
         FROM stars s2
         WHERE s2.galaxy_id = ${alias}.galaxy_id
         AND s2.spectr = %s
         AND s2.id != ${alias}.id
         ORDER BY (POW(s2.position_x - ${alias}.position_x, 2) +
                   POW(s2.position_y - ${alias}.position_y, 2) +
                   POW(s2.position_z - ${alias}.position_z, 2)) ASC
         LIMIT 1)
        `;
        return { sql, params: [spectrVal] };
    }
}

class XDistRule extends AmountRule {
    constructor(all = false) {
        super();
        this.all = all;
    }
    toSql(alias = "s") {
        return { sql: `${alias}.dist_X`, params: [] };
    }
}

class PlanetWaterIdRule extends AmountRule {
    toSql(alias = "p") {
        return { sql: `${alias}.water_item`, params: [] };
    }
}

class GasGiantRule extends GenericRule {
    constructor(iceGiants = null) {
        super();
        this.ice = iceGiants;
    }
    toSql(alias = "p") {
        if (this.ice === null) return { sql: `${alias}.gas_giant = %s`, params: [true] };
        else if (this.ice === false) return { sql: `(${alias}.gas_giant = %s AND ${alias}.temperature >= %s)`, params: [true, 0.0] };
        else if (this.ice === true)  return { sql: `(${alias}.gas_giant = %s AND ${alias}.temperature < %s)`,  params: [true, 0.0] };
        else throw new TypeError("this.ice was neither true, false or null");
    }
}

class GasRateRule extends AmountRule {
    constructor(gasType) {
        super();
        if (typeof gasType === 'number') {
            if (![1120, 1121, 1011].includes(gasType)) throw new Error(`Gas Type can only be 1120 (Hydrogen), 1121 (Deuterium) or 1011 (Fireice)! It was ${gasType}`);
            this.gasType = {1120: 'h', 1121: 'd', 1011: 'i'}[gasType];
        } else if (typeof gasType === 'string') {
            if (gasType && gasType.length === 1) this.gasType = gasType.toLowerCase();
            else {
                this.gasType = {
                    "Hydrogen": "h",
                    "Deuterium": "d",
                    "Fireice": "i"
                }[gasType] || "h";
            }
        } else {
             this.gasType = "h";
        }
    }
    toSql(alias = "p") {
        return { sql: `${alias}.gas_${this.gasType}`, params: [] };
    }
}

class PlanetSunDistanceRule extends AmountRule {
    toSql(alias = "p") {
        return { sql: `${alias}.sun_distance`, params: [] };
    }
}

class PlanetInsideDysonRule extends GenericRule {
    toSql(alias = "p") {
        return { sql: `${alias}.inside_ds`, params: [] };
    }
}

class SatelliteCountRule extends AmountRule {
    toSql(alias = "p") {
        const sql = `
        (SELECT COUNT(*) FROM planets p2
         WHERE p2.star_id = ${alias}.star_id
         AND p2.orbiting = ${alias}.index)
        `;
        return { sql, params: [] };
    }
}

class TidalLockRule extends GenericRule {
    toSql(alias = "p") {
        return { sql: `${alias}.tidal_lock`, params: [] };
    }
}

class PlanetCountRule extends AmountRule {
    constructor(planetRule = null) {
        super();
        this.planetRule = planetRule;
    }
    toSql(alias = "s") {
        if (this.planetRule) {
            const { sql: ruleSql, params: ruleParams } = this.planetRule.toSql("p");
            const sql = `(SELECT COUNT(*) FROM planets p WHERE p.star_id = ${alias}.id AND ${ruleSql})`;
            return { sql, params: ruleParams };
        } else {
            const sql = `(SELECT COUNT(*) FROM planets p WHERE p.star_id = ${alias}.id)`;
            return { sql, params: [] };
        }
    }
}

class HasPlanetRule extends GenericRule {
    constructor(planetRule) {
        super();
        this.planetRule = planetRule;
    }
    toSql(alias = "s") {
        let innerSql, innerParams;
        if (this.planetRule instanceof GenericRule) {
            const result = this.planetRule.toSql("p");
            innerSql = result.sql;
            innerParams = result.params;
        } else {
            [innerSql, innerParams] = this.planetRule;
        }
        const sql = `EXISTS (SELECT 1 FROM planets p WHERE p.star_id = ${alias}.id AND ${innerSql})`;
        return { sql, params: innerParams };
    }
}

// Added ComparisonRule to handle the SQLOperator.sql logic which was used in Python but not explicitly as a class
class ComparisonRule extends GenericRule {
    constructor(rule, operand, amount) {
        super();
        this.rule = rule;
        this.op = operand;
        this.amount = amount;
    }
    toSql(alias = "s") {
        return getSqlWithOperator(this.op, this.rule, this.amount);
    }
}

// Map for safe class instantiation
const RULE_CLASSES = {
    StarAmountRule,
    TotalAmountRule,
    StarVeinRule,
    PlanetVeinRule,
    AvgVeinRule,
    StartDistanceRule,
    BirthRule,
    ThemeRule,
    NotRule,
    AndRule,
    OrRule,
    StarLuminosityRule,
    DysonRadiusRule,
    StarTypeRule,
    StarSpectrRule,
    DistanceToSpectrRule,
    XDistRule,
    PlanetWaterIdRule,
    GasGiantRule,
    GasRateRule,
    PlanetSunDistanceRule,
    PlanetInsideDysonRule,
    SatelliteCountRule,
    TidalLockRule,
    PlanetCountRule,
    HasPlanetRule,
    ComparisonRule
};

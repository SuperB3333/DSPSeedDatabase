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

class GenericRule {
    toSql(alias = "s") {
        throw new Error("NotImplementedError");
    }
}

// AmountRule returns a numeric expression.
// If op and amount are present, it returns a comparison (boolean).
class AmountRule extends GenericRule {
    constructor(operand = null, amount = null) {
        super();
        this.op = operand;
        this.amount = amount;
    }

    toSqlRaw(alias = "s") {
        throw new Error("NotImplementedError");
    }

    toSql(alias = "s") {
        const raw = this.toSqlRaw(alias);
        if (this.op && this.amount !== null) {
            const opValue = SQLOperator[this.op] || this.op;
            return {
                sql: `(${raw.sql} ${opValue} %s)`,
                params: [...raw.params, this.amount]
            };
        }
        return raw;
    }
}

// --- Query Rules ---

class StarAmountRule extends GenericRule {
    constructor(ruleset, amountStars, operand) {
        super();
        this.rule = ruleset;
        this.amount = amountStars;
        this.op = operand;
    }
    toSql(alias = 's') {
        let where, params;
        const result = this.rule.toSql('s');
        where = result.sql;
        params = result.params;

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

class BaseCombiningRule extends GenericRule {
    constructor(queries, keyword) {
        super();
        this.queries = queries || [];
        this.keyword = keyword;
    }
    toSql(alias = "s") {
        if (!this.queries || this.queries.length === 0) {
            return { sql: "SELECT NULL LIMIT 0", params: [] };
        }
        const clauses = [];
        const params = [];
        for (const q of this.queries) {
            const res = q.toSql(alias);
            clauses.push(`(${res.sql})`);
            params.push(...res.params);
        }
        return { sql: clauses.join(` ${this.keyword} `), params };
    }
}

class UnionRule extends BaseCombiningRule {
    constructor(queries) {
        super(queries, "UNION");
    }
}

class IntersectRule extends BaseCombiningRule {
    constructor(queries) {
        super(queries, "INTERSECT");
    }
}

class ExceptRule extends BaseCombiningRule {
    constructor(queries) {
        super(queries, "EXCEPT");
    }
}

// --- Boolean Rules ---

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
            const { sql: s, params: p } = rule.toSql(alias);
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
            const { sql: s, params: p } = rule.toSql(alias);
            clauses.push(`(${s})`);
            params.push(...p);
        }
        return { sql: clauses.join(" OR "), params };
    }
}

class NotRule extends GenericRule {
    constructor(rule) {
        super();
        this.rule = rule;
    }
    toSql(alias = "s") {
        const { sql, params } = this.rule.toSql(alias);
        return { sql: `NOT (${sql})`, params };
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

class PlanetInsideDysonRule extends GenericRule {
    toSql(alias = "p") {
        return { sql: `${alias}.inside_ds`, params: [] };
    }
}

class TidalLockRule extends GenericRule {
    toSql(alias = "p") {
        return { sql: `${alias}.tidal_lock`, params: [] };
    }
}

class HasPlanetRule extends GenericRule {
    constructor(planetRule) {
        super();
        this.planetRule = planetRule;
    }
    toSql(alias = "s") {
        const { sql: innerSql, params: innerParams } = this.planetRule.toSql("p");
        const sql = `EXISTS (SELECT 1 FROM planets p WHERE p.star_id = ${alias}.id AND ${innerSql})`;
        return { sql, params: innerParams };
    }
}

// --- Amount Rules ---

class TotalAmountRule extends AmountRule {
    constructor(planetaryRule, operand = null, amount = null) {
        super(operand, amount);
        this.rule = planetaryRule;
    }
    toSqlRaw(alias = "s") {
        const { sql: ruleSql, params } = this.rule.toSqlRaw("p");
        const sql = `(
            SELECT COALESCE(SUM(${ruleSql}), 0)
            FROM planets p
            WHERE p.star_id = ${alias}.id
        )`;
        return { sql, params };
    }
}

class StarVeinRule extends AmountRule {
    constructor(veinType, operand = null, amount = null) {
        super(operand, amount);
        this.veinType = (veinType || 'Iron').toLowerCase();
    }
    toSqlRaw(alias = "s") {
        return { sql: `${alias}.ore_${this.veinType}`, params: [] };
    }
}

class PlanetVeinRule extends AmountRule {
    constructor(veinType, operand = null, amount = null) {
        super(operand, amount);
        this.veinType = (veinType || 'Iron').toLowerCase();
    }
    toSqlRaw(alias = "p") {
        return { sql: `${alias}.estimate_${this.veinType}`, params: [] };
    }
}

class AvgVeinRule extends AmountRule {
    constructor(veinType, operand = null, amount = null) {
        super(operand, amount);
        this.veinType = veinType;
    }
    toSqlRaw(alias = "s") {
        if (alias === "s") return { sql: `${alias}.ore_${this.veinType.toLowerCase()}`, params: [] };
        else return { sql: `${alias}.estimate_${this.veinType.toLowerCase()}`, params: [] };
    }
}

class StartDistanceRule extends AmountRule {
    toSqlRaw(alias = "s") {
        return { sql: `${alias}.start_dist`, params: [] };
    }
}

class StarLuminosityRule extends AmountRule {
    toSqlRaw(alias = "s") {
        return { sql: `${alias}.luminosity`, params: [] };
    }
}

class DysonRadiusRule extends AmountRule {
    toSqlRaw(alias = "s") {
        return { sql: `${alias}.dyson_radius`, params: [] };
    }
}

class DistanceToSpectrRule extends AmountRule {
    constructor(spectr, operand = null, amount = null) {
        super(operand, amount);
        this.spectr = spectr;
    }
    toSqlRaw(alias = "s") {
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
    constructor(all = false, operand = null, amount = null) {
        super(operand, amount);
        this.all = all;
    }
    toSqlRaw(alias = "s") {
        return { sql: `${alias}.dist_X`, params: [] };
    }
}

class PlanetWaterIdRule extends AmountRule {
    toSqlRaw(alias = "p") {
        return { sql: `${alias}.water_item`, params: [] };
    }
}

class GasRateRule extends AmountRule {
    constructor(gasType, operand = null, amount = null) {
        super(operand, amount);
        const mapping = { "Hydrogen": "h", "Deuterium": "d", "Fireice": "i", 1120: "h", 1121: "d", 1011: "i" };
        this.gasType = mapping[gasType] || "h";
    }
    toSqlRaw(alias = "p") {
        return { sql: `${alias}.gas_${this.gasType}`, params: [] };
    }
}

class PlanetSunDistanceRule extends AmountRule {
    toSqlRaw(alias = "p") {
        return { sql: `${alias}.sun_distance`, params: [] };
    }
}

class SatelliteCountRule extends AmountRule {
    toSqlRaw(alias = "p") {
        const sql = `
        (SELECT COUNT(*) FROM planets p2
         WHERE p2.star_id = ${alias}.star_id
         AND p2.orbiting = ${alias}.index)
        `;
        return { sql, params: [] };
    }
}

class PlanetCountRule extends AmountRule {
    constructor(planetRule = null, operand = null, amount = null) {
        super(operand, amount);
        this.planetRule = planetRule;
    }
    toSqlRaw(alias = "s") {
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

// Map for safe class instantiation
const RULE_CLASSES = {
    StarAmountRule,
    UnionRule,
    IntersectRule,
    ExceptRule,
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
    HasPlanetRule
};

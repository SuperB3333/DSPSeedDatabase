const VEINS = ["Iron", "Copper", "Silicium", "Titanium", "Stone", "Coal", "Oil", "Fireice", "Diamond", "Fractal", "Crysrub", "Grat", "Bamboo", "Mag"];
const STAR_TYPES = ["MainSeqStar", "GiantStar", "WhiteDwarf", "NeutronStar", "BlackHole"];
const SPECTR_TYPES = ["M", "K", "G", "F", "A", "B", "O", "X"];
const SQL_OPERATORS = [
    { value: "e", label: "=" },
    { value: "ne", label: "!=" },
    { value: "gte", label: ">=" },
    { value: "lte", label: "<=" },
    { value: "gt", label: ">" },
    { value: "lt", label: "<" }
];
const GAS_TYPES = ["Hydrogen", "Deuterium", "Fireice"];

const AMOUNT_PARAMS = [
    { name: "operand", type: "enum", options: SQL_OPERATORS, description: "Comparison operator", optional: true },
    { name: "amount", type: "number", description: "Constant value", optional: true }
];

const RULE_METADATA = {
    // --- Query Rules ---
    StarAmountRule: {
        name: "Star Amount Rule (Seed Finder)",
        params: [
            { name: "ruleset", type: "boolean", description: "Filter for stars" },
            { name: "amountStars", type: "number", description: "Number of stars" },
            { name: "operand", type: "enum", options: SQL_OPERATORS, description: "Comparison operator" }
        ],
        category: "query"
    },
    UnionRule: {
        name: "UNION (Combine Queries)",
        params: [
            { name: "queries", type: "queries", description: "Child queries" }
        ],
        category: "query"
    },
    IntersectRule: {
        name: "INTERSECT (Combine Queries)",
        params: [
            { name: "queries", type: "queries", description: "Child queries" }
        ],
        category: "query"
    },
    ExceptRule: {
        name: "EXCEPT (Combine Queries)",
        params: [
            { name: "queries", type: "queries", description: "Child queries" }
        ],
        category: "query"
    },

    // --- Boolean Rules ---
    AndRule: {
        name: "AND",
        params: [
            { name: "rules", type: "booleans", description: "Child rules" }
        ],
        category: "boolean"
    },
    OrRule: {
        name: "OR",
        params: [
            { name: "rules", type: "booleans", description: "Child rules" }
        ],
        category: "boolean"
    },
    NotRule: {
        name: "NOT",
        params: [
            { name: "rule", type: "boolean", description: "Child rule" }
        ],
        category: "boolean"
    },
    StarTypeRule: {
        name: "Star Type",
        params: [
            { name: "starType", type: "enum", options: STAR_TYPES, description: "Type of star" }
        ],
        category: "boolean"
    },
    StarSpectrRule: {
        name: "Star Spectral Type",
        params: [
            { name: "spectr", type: "enum", options: SPECTR_TYPES, description: "Spectral type" }
        ],
        category: "boolean"
    },
    BirthRule: {
        name: "Birth Star/Planet",
        params: [],
        category: "boolean"
    },
    ThemeRule: {
        name: "Planet Theme",
        params: [
            { name: "targetIds", type: "number_list", description: "Theme IDs (comma separated)" }
        ],
        category: "boolean"
    },
    GasGiantRule: {
        name: "Gas Giant",
        params: [
            { name: "iceGiants", type: "enum", options: [
                { value: null, label: "Any Gas Giant" },
                { value: false, label: "Gas Giant (Warm)" },
                { value: true, label: "Ice Giant (Cold)" }
            ], description: "Type of gas giant" }
        ],
        category: "boolean"
    },
    PlanetInsideDysonRule: {
        name: "Planet Inside Dyson Sphere",
        params: [],
        category: "boolean"
    },
    TidalLockRule: {
        name: "Tidally Locked Planet",
        params: [],
        category: "boolean"
    },
    HasPlanetRule: {
        name: "Has Planet Matching",
        params: [
            { name: "planetRule", type: "boolean", description: "Filter for planets" }
        ],
        category: "boolean"
    },

    // --- Amount Rules ---
    StarVeinRule: {
        name: "Star Vein Amount",
        params: [
            { name: "veinType", type: "enum", options: VEINS, description: "Vein type" },
            ...AMOUNT_PARAMS
        ],
        categories: ["boolean", "amount"]
    },
    PlanetVeinRule: {
        name: "Planet Vein Amount",
        params: [
            { name: "veinType", type: "enum", options: VEINS, description: "Vein type" },
            ...AMOUNT_PARAMS
        ],
        categories: ["boolean", "amount"]
    },
    AvgVeinRule: {
        name: "Average Vein Amount",
        params: [
            { name: "veinType", type: "enum", options: VEINS, description: "Vein type" },
            ...AMOUNT_PARAMS
        ],
        categories: ["boolean", "amount"]
    },
    TotalAmountRule: {
        name: "Total Amount on Star (from planets)",
        params: [
            { name: "planetaryRule", type: "amount", description: "Planetary amount rule" },
            ...AMOUNT_PARAMS
        ],
        categories: ["boolean", "amount"]
    },
    StartDistanceRule: {
        name: "Distance from Start",
        params: [...AMOUNT_PARAMS],
        categories: ["boolean", "amount"]
    },
    StarLuminosityRule: {
        name: "Star Luminosity",
        params: [...AMOUNT_PARAMS],
        categories: ["boolean", "amount"]
    },
    DysonRadiusRule: {
        name: "Dyson Sphere Radius",
        params: [...AMOUNT_PARAMS],
        categories: ["boolean", "amount"]
    },
    DistanceToSpectrRule: {
        name: "Distance to nearest Spectral Type",
        params: [
            { name: "spectr", type: "enum", options: SPECTR_TYPES, description: "Spectral type" },
            ...AMOUNT_PARAMS
        ],
        categories: ["boolean", "amount"]
    },
    XDistRule: {
        name: "X Distance",
        params: [
            { name: "all", type: "bool", description: "All?" },
            ...AMOUNT_PARAMS
        ],
        categories: ["boolean", "amount"]
    },
    PlanetWaterIdRule: {
        name: "Planet Water Item ID",
        params: [...AMOUNT_PARAMS],
        categories: ["boolean", "amount"]
    },
    GasRateRule: {
        name: "Gas Harvesting Rate",
        params: [
            { name: "gasType", type: "enum", options: GAS_TYPES, description: "Gas type" },
            ...AMOUNT_PARAMS
        ],
        categories: ["boolean", "amount"]
    },
    PlanetSunDistanceRule: {
        name: "Planet Distance from Sun",
        params: [...AMOUNT_PARAMS],
        categories: ["boolean", "amount"]
    },
    SatelliteCountRule: {
        name: "Satellite Count",
        params: [...AMOUNT_PARAMS],
        categories: ["boolean", "amount"]
    },
    PlanetCountRule: {
        name: "Planet Count",
        params: [
            { name: "planetRule", type: "boolean_optional", description: "Filter for planets (optional)" },
            ...AMOUNT_PARAMS
        ],
        categories: ["boolean", "amount"]
    }
};

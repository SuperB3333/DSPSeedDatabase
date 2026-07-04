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

const RULE_METADATA = {
    StarAmountRule: {
        name: "Star Amount Rule (Top Level)",
        params: [
            { name: "ruleset", type: "rule", description: "A rule to filter stars" },
            { name: "amountStars", type: "number", description: "Number of stars" },
            { name: "operand", type: "enum", options: SQL_OPERATORS, description: "Comparison operator" }
        ],
        category: "generic"
    },
    AndRule: {
        name: "AND (Logic)",
        params: [
            { name: "rules", type: "rules", description: "Child rules" }
        ],
        category: "generic"
    },
    OrRule: {
        name: "OR (Logic)",
        params: [
            { name: "rules", type: "rules", description: "Child rules" }
        ],
        category: "generic"
    },
    NotRule: {
        name: "NOT (Logic)",
        params: [
            { name: "rule", type: "rule", description: "Child rule" }
        ],
        category: "generic"
    },
    ComparisonRule: {
        name: "Comparison (Value vs Number)",
        params: [
            { name: "rule", type: "amount_rule", description: "Value to compare" },
            { name: "operand", type: "enum", options: SQL_OPERATORS, description: "Operator" },
            { name: "amount", type: "number", description: "Constant value" }
        ],
        category: "generic"
    },
    StarTypeRule: {
        name: "Star Type",
        params: [
            { name: "starType", type: "enum", options: STAR_TYPES, description: "Type of star" }
        ],
        category: "generic"
    },
    StarSpectrRule: {
        name: "Star Spectral Type",
        params: [
            { name: "spectr", type: "enum", options: SPECTR_TYPES, description: "Spectral type" }
        ],
        category: "generic"
    },
    BirthRule: {
        name: "Birth Star/Planet",
        params: [],
        category: "generic"
    },
    ThemeRule: {
        name: "Planet Theme",
        params: [
            { name: "targetIds", type: "number_list", description: "Theme IDs (comma separated)" }
        ],
        category: "generic"
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
        category: "generic"
    },
    PlanetInsideDysonRule: {
        name: "Planet Inside Dyson Sphere",
        params: [],
        category: "generic"
    },
    TidalLockRule: {
        name: "Tidally Locked Planet",
        params: [],
        category: "generic"
    },
    HasPlanetRule: {
        name: "Has Planet Matching",
        params: [
            { name: "planetRule", type: "rule", description: "Filter for planets" }
        ],
        category: "generic"
    },
    // Amount Rules
    StarVeinRule: {
        name: "Star Vein Amount",
        params: [
            { name: "veinType", type: "enum", options: VEINS, description: "Vein type" }
        ],
        category: "amount"
    },
    PlanetVeinRule: {
        name: "Planet Vein Amount",
        params: [
            { name: "veinType", type: "enum", options: VEINS, description: "Vein type" }
        ],
        category: "amount"
    },
    AvgVeinRule: {
        name: "Average Vein Amount",
        params: [
            { name: "veinType", type: "enum", options: VEINS, description: "Vein type" }
        ],
        category: "amount"
    },
    TotalAmountRule: {
        name: "Total Amount on Star (from planets)",
        params: [
            { name: "planetaryRule", type: "amount_rule", description: "Planetary amount rule" }
        ],
        category: "amount"
    },
    StartDistanceRule: {
        name: "Distance from Start",
        params: [],
        category: "amount"
    },
    StarLuminosityRule: {
        name: "Star Luminosity",
        params: [],
        category: "amount"
    },
    DysonRadiusRule: {
        name: "Dyson Sphere Radius",
        params: [],
        category: "amount"
    },
    DistanceToSpectrRule: {
        name: "Distance to nearest Spectral Type",
        params: [
            { name: "spectr", type: "enum", options: SPECTR_TYPES, description: "Spectral type" }
        ],
        category: "amount"
    },
    XDistRule: {
        name: "X Distance",
        params: [
            { name: "all", type: "boolean", description: "All?" }
        ],
        category: "amount"
    },
    PlanetWaterIdRule: {
        name: "Planet Water Item ID",
        params: [],
        category: "amount"
    },
    GasRateRule: {
        name: "Gas Harvesting Rate",
        params: [
            { name: "gasType", type: "enum", options: GAS_TYPES, description: "Gas type" }
        ],
        category: "amount"
    },
    PlanetSunDistanceRule: {
        name: "Planet Distance from Sun",
        params: [],
        category: "amount"
    },
    SatelliteCountRule: {
        name: "Satellite Count",
        params: [],
        category: "amount"
    },
    PlanetCountRule: {
        name: "Planet Count",
        params: [
            { name: "planetRule", type: "rule_optional", description: "Filter for planets (optional)" }
        ],
        category: "amount"
    }
};

const STORAGE_KEY = 'dsp_ruleset';

function saveRuleset(ruleset) {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(ruleset));
}

function loadRuleset() {
    const data = localStorage.getItem(STORAGE_KEY);
    if (!data) return null;
    try {
        return JSON.parse(data);
    } catch (e) {
        console.error("Failed to parse ruleset from localStorage", e);
        return null;
    }
}

function clearRuleset() {
    localStorage.removeItem(STORAGE_KEY);
}

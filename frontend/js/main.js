const DEFAULT_RULESET = {
    type: 'StarAmountRule',
    params: {
        ruleset: {
            type: 'AndRule',
            params: {
                rules: [
                    {
                        type: 'StarSpectrRule',
                        params: { spectr: 'O' }
                    },
                    {
                        type: 'StarTypeRule',
                        params: { starType: 'GiantStar' }
                    }
                ]
            }
        },
        amountStars: 2,
        operand: 'gte'
    }
};

function init() {
    window.ruleset = loadRuleset() || DEFAULT_RULESET;
    updateUI();
}

function clearAndReset() {
    if (confirm("Are you sure you want to reset the ruleset?")) {
        clearRuleset();
        window.ruleset = JSON.parse(JSON.stringify(DEFAULT_RULESET));
        updateUI();
    }
}

document.addEventListener('DOMContentLoaded', init);

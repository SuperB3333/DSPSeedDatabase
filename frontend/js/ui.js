function getRulesByCategory(category) {
    return Object.entries(RULE_METADATA)
        .filter(([_, meta]) => {
            if (meta.category === category) return true;
            return !!(meta.categories && meta.categories.includes(category));

        })
        .map(([type, _]) => type);
}

function renderRule(ruleData, parent = null, keyOrIndex = null, expectedCategory = 'query', isNestedAmount = false) {
    const container = document.createElement('div');
    container.className = 'rule-node';

    const header = document.createElement('div');
    header.className = 'rule-header';

    const select = document.createElement('select');
    select.className = 'rule-type-select';

    const allowedTypes = getRulesByCategory(expectedCategory);

    allowedTypes.forEach(type => {
        const opt = document.createElement('option');
        opt.value = type;
        opt.textContent = RULE_METADATA[type].name;
        if (type === ruleData.type) opt.selected = true;
        select.appendChild(opt);
    });

    select.onchange = () => {
        const newType = select.value;
        const newMeta = RULE_METADATA[newType];
        ruleData.type = newType;
        ruleData.params = {};
        // Initialize default params
        newMeta.params.forEach(p => {
            if (p.type === 'queries') ruleData.params[p.name] = [];
            else if (p.type === 'booleans') ruleData.params[p.name] = [];
            else if (p.type === 'boolean') ruleData.params[p.name] = { type: 'AndRule', params: { rules: [] } };
            else if (p.type === 'amount') ruleData.params[p.name] = { type: 'StarVeinRule', params: { veinType: 'Iron' } };
            else if (p.type === 'query') ruleData.params[p.name] = { type: 'StarAmountRule', params: { ruleset: { type: 'AndRule', params: { rules: [] } }, amountStars: 1, operand: 'gte' } };
            else if (p.type === 'boolean_optional') ruleData.params[p.name] = null;
            else if (p.type === 'enum') ruleData.params[p.name] = p.options[0].value !== undefined ? p.options[0].value : p.options[0];
            else if (p.type === 'number') ruleData.params[p.name] = 0;
            else if (p.type === 'bool') ruleData.params[p.name] = false;
            else if (p.type === 'number_list') ruleData.params[p.name] = [];
        });
        updateUI();
    };

    header.appendChild(select);

    const isRoot = parent === null;
    const isInList = Array.isArray(parent);

    let isOptional = false;
    if (parent && !isInList) {
        const parentMeta = RULE_METADATA[parent.type];
        const pMeta = parentMeta.params.find(p => p.name === keyOrIndex);
        if (pMeta && pMeta.type === 'boolean_optional') isOptional = true;
    }

    if (!isRoot && (isInList || isOptional)) {
        const deleteBtn = document.createElement('button');
        deleteBtn.textContent = 'Delete';
        deleteBtn.className = 'delete-btn';
        deleteBtn.onclick = () => {
            if (isInList) {
                parent.splice(keyOrIndex, 1);
            } else {
                parent.params[keyOrIndex] = null;
            }
            updateUI();
        };
        header.appendChild(deleteBtn);
    }

    container.appendChild(header);

    const paramsContainer = document.createElement('div');
    paramsContainer.className = 'rule-params';

    const meta = RULE_METADATA[ruleData.type];
    meta.params.forEach(paramMeta => {
        // Hide operand and amount if we are in a nested amount context
        if (isNestedAmount && (paramMeta.name === 'operand' || paramMeta.name === 'amount')) {
            return;
        }

        const paramDiv = document.createElement('div');
        paramDiv.className = 'param-row';
        const label = document.createElement('label');
        label.textContent = paramMeta.description || paramMeta.name;
        paramDiv.appendChild(label);

        const val = ruleData.params[paramMeta.name];

        if (paramMeta.type === 'number') {
            const input = document.createElement('input');
            input.type = 'number';
            input.value = val === undefined ? 0 : val;
            input.oninput = () => {
                ruleData.params[paramMeta.name] = parseFloat(input.value);
                saveRuleset(window.ruleset);
            };
            paramDiv.appendChild(input);
        } else if (paramMeta.type === 'bool') {
            const input = document.createElement('input');
            input.type = 'checkbox';
            input.checked = !!val;
            input.onchange = () => {
                ruleData.params[paramMeta.name] = input.checked;
                saveAndRefresh();
            };
            paramDiv.appendChild(input);
        } else if (paramMeta.type === 'enum') {
            const input = document.createElement('select');
            paramMeta.options.forEach(opt => {
                const o = document.createElement('option');
                if (typeof opt === 'string') {
                    o.value = o.textContent = opt;
                    if (val === opt) o.selected = true;
                } else {
                    o.value = opt.value === null ? "null" : opt.value;
                    o.textContent = opt.label;
                    if (val === opt.value) o.selected = true;
                }
                input.appendChild(o);
            });
            input.onchange = () => {
                let v = input.value;
                if (v === "null") v = null;
                else if (v === "true") v = true;
                else if (v === "false") v = false;
                ruleData.params[paramMeta.name] = v;
                saveAndRefresh();
            };
            paramDiv.appendChild(input);
        } else if (paramMeta.type === 'number_list') {
            const input = document.createElement('input');
            input.type = 'text';
            input.value = Array.isArray(val) ? val.join(', ') : '';
            input.oninput = () => {
                ruleData.params[paramMeta.name] = input.value.split(',').map(s => parseInt(s.trim())).filter(n => !isNaN(n));
                saveRuleset(window.ruleset);
            };
            paramDiv.appendChild(input);
        } else if (paramMeta.type === 'queries') {
            const listContainer = document.createElement('div');
            listContainer.className = 'child-rules-list';
            (val || []).forEach((childRule, i) => {
                listContainer.appendChild(renderRule(childRule, val, i, 'query'));
            });
            const addBtn = document.createElement('button');
            addBtn.textContent = 'Add Query';
            addBtn.onclick = () => {
                if (!ruleData.params[paramMeta.name]) ruleData.params[paramMeta.name] = [];
                ruleData.params[paramMeta.name].push({ type: 'StarAmountRule', params: { ruleset: { type: 'AndRule', params: { rules: [] } }, amountStars: 1, operand: 'gte' } });
                updateUI();
            };
            paramDiv.appendChild(listContainer);
            paramDiv.appendChild(addBtn);
        } else if (paramMeta.type === 'booleans') {
            const listContainer = document.createElement('div');
            listContainer.className = 'child-rules-list';
            (val || []).forEach((childRule, i) => {
                listContainer.appendChild(renderRule(childRule, val, i, 'boolean'));
            });
            const addBtn = document.createElement('button');
            addBtn.textContent = 'Add Rule';
            addBtn.onclick = () => {
                if (!ruleData.params[paramMeta.name]) ruleData.params[paramMeta.name] = [];
                ruleData.params[paramMeta.name].push({ type: 'AndRule', params: { rules: [] } });
                updateUI();
            };
            paramDiv.appendChild(listContainer);
            paramDiv.appendChild(addBtn);
        } else if (paramMeta.type === 'query') {
            paramDiv.appendChild(renderRule(val, ruleData, paramMeta.name, 'query'));
        } else if (paramMeta.type === 'boolean') {
            paramDiv.appendChild(renderRule(val, ruleData, paramMeta.name, 'boolean'));
        } else if (paramMeta.type === 'amount') {
            paramDiv.appendChild(renderRule(val, ruleData, paramMeta.name, 'amount', true));
        } else if (paramMeta.type === 'boolean_optional') {
            if (val === null) {
                const addBtn = document.createElement('button');
                addBtn.textContent = 'Add Optional Rule';
                addBtn.onclick = () => {
                    ruleData.params[paramMeta.name] = { type: 'AndRule', params: { rules: [] } };
                    updateUI();
                };
                paramDiv.appendChild(addBtn);
            } else {
                paramDiv.appendChild(renderRule(val, ruleData, paramMeta.name, 'boolean'));
            }
        }

        paramsContainer.appendChild(paramDiv);
    });

    container.appendChild(paramsContainer);

    return container;
}

function updateUI() {
    const rootContainer = document.getElementById('rule-tree-container');
    rootContainer.innerHTML = '';
    rootContainer.appendChild(renderRule(window.ruleset, null, null, 'query'));
    saveRuleset(window.ruleset);
}

function saveAndRefresh() {
    saveRuleset(window.ruleset);
    updateUI();
}

function instantiateRule(ruleData) {
    if (!ruleData) return null;
    const meta = RULE_METADATA[ruleData.type];
    if (!meta) throw new Error(`Unknown rule type: ${ruleData.type}`);

    const args = [];
    meta.params.forEach(p => {
        const val = ruleData.params[p.name];
        if (['queries', 'booleans'].includes(p.type)) {
            args.push((val || []).map(child => instantiateRule(child)));
        } else if (['query', 'boolean', 'amount'].includes(p.type)) {
            args.push(instantiateRule(val));
        } else if (p.type === 'boolean_optional') {
            args.push(val ? instantiateRule(val) : null);
        } else {
            args.push(val);
        }
    });

    const ruleClass = RULE_CLASSES[ruleData.type];
    if (!ruleClass) throw new Error(`Class not found for type: ${ruleData.type}`);
    return new ruleClass(...args);
}

function compileRuleset() {
    try {
        const rootRule = instantiateRule(window.ruleset);
        const result = rootRule.toSql();
        document.getElementById('sql-output').textContent = dedent(result.sql);
        document.getElementById('params-output').textContent = JSON.stringify(result.params, null, 2);
    } catch (e) {
        console.error(e);
        document.getElementById('sql-output').textContent = "Compilation Error: " + e.message;
        document.getElementById('params-output').textContent = "";
    }
}
function dedent(sql) {
    return sql
        .toString()
        .split("\n")
        .map(function(line){ return line.trim(); }, undefined)
        .filter(function(line) { return line.length !== 0; })
        .join("\n")
}
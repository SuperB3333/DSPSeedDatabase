function renderRule(ruleData, parent = null, keyOrIndex = null) {
    const container = document.createElement('div');
    container.className = 'rule-node';

    const header = document.createElement('div');
    header.className = 'rule-header';

    const select = document.createElement('select');
    select.className = 'rule-type-select';

    const categories = {
        "Generic Rules": [],
        "Amount Rules": []
    };

    for (const [type, meta] of Object.entries(RULE_METADATA)) {
        if (meta.category === 'amount') categories["Amount Rules"].push(type);
        else categories["Generic Rules"].push(type);
    }

    for (const [catName, types] of Object.entries(categories)) {
        const group = document.createElement('optgroup');
        group.label = catName;
        types.forEach(type => {
            const opt = document.createElement('option');
            opt.value = type;
            opt.textContent = RULE_METADATA[type].name;
            if (type === ruleData.type) opt.selected = true;
            group.appendChild(opt);
        });
        select.appendChild(group);
    }

    select.onchange = () => {
        const newType = select.value;
        const newMeta = RULE_METADATA[newType];
        ruleData.type = newType;
        ruleData.params = {};
        // Initialize default params
        newMeta.params.forEach(p => {
            if (p.type === 'rules') ruleData.params[p.name] = [];
            else if (p.type === 'rule') ruleData.params[p.name] = { type: 'AndRule', params: { rules: [] } };
            else if (p.type === 'amount_rule') ruleData.params[p.name] = { type: 'StarVeinRule', params: { veinType: 'Iron' } };
            else if (p.type === 'rule_optional') ruleData.params[p.name] = null;
            else if (p.type === 'enum') ruleData.params[p.name] = p.options[0].value !== undefined ? p.options[0].value : p.options[0];
            else if (p.type === 'number') ruleData.params[p.name] = 0;
            else if (p.type === 'boolean') ruleData.params[p.name] = false;
            else if (p.type === 'number_list') ruleData.params[p.name] = [];
        });
        updateUI();
    };

    header.appendChild(select);

    if (parent !== null) {
        const deleteBtn = document.createElement('button');
        deleteBtn.textContent = 'Delete';
        deleteBtn.className = 'delete-btn';
        deleteBtn.onclick = () => {
            if (Array.isArray(parent)) {
                parent.splice(keyOrIndex, 1);
            } else {
                // If it's a mandatory named parameter, we probably shouldn't "delete" it,
                // but for flexibility we can reset it to a default or null if optional.
                // However, based on the requirement to delete rules, we'll allow it if it's in a list.
                // For named params like 'ruleset' in StarAmountRule, maybe we just reset it.
                // Let's check metadata if it's optional.
                const parentRuleMeta = RULE_METADATA[parent.type];
                const paramMeta = parentRuleMeta.params.find(p => p.name === keyOrIndex);
                if (paramMeta.type === 'rule_optional') {
                    parent.params[keyOrIndex] = null;
                } else if (paramMeta.type === 'rule') {
                    parent.params[keyOrIndex] = { type: 'AndRule', params: { rules: [] } };
                } else if (paramMeta.type === 'amount_rule') {
                    parent.params[keyOrIndex] = { type: 'StarVeinRule', params: { veinType: 'Iron' } };
                }
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
        const paramDiv = document.createElement('div');
        paramDiv.className = 'param-row';
        const label = document.createElement('label');
        label.textContent = paramMeta.description || paramMeta.name;
        paramDiv.appendChild(label);

        const val = ruleData.params[paramMeta.name];

        if (paramMeta.type === 'number') {
            const input = document.createElement('input');
            input.type = 'number';
            input.value = val;
            input.oninput = () => {
                ruleData.params[paramMeta.name] = parseFloat(input.value);
                saveRuleset(window.ruleset);
            };
            paramDiv.appendChild(input);
        } else if (paramMeta.type === 'boolean') {
            const input = document.createElement('input');
            input.type = 'checkbox';
            input.checked = val;
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
        } else if (paramMeta.type === 'rules') {
            const listContainer = document.createElement('div');
            listContainer.className = 'child-rules-list';
            val.forEach((childRule, i) => {
                listContainer.appendChild(renderRule(childRule, val, i));
            });
            const addBtn = document.createElement('button');
            addBtn.textContent = 'Add Child Rule';
            addBtn.onclick = () => {
                val.push({ type: 'AndRule', params: { rules: [] } });
                updateUI();
            };
            paramDiv.appendChild(listContainer);
            paramDiv.appendChild(addBtn);
        } else if (paramMeta.type === 'rule' || paramMeta.type === 'amount_rule') {
            paramDiv.appendChild(renderRule(val, ruleData, paramMeta.name));
        } else if (paramMeta.type === 'rule_optional') {
            if (val === null) {
                const addBtn = document.createElement('button');
                addBtn.textContent = 'Add Optional Rule';
                addBtn.onclick = () => {
                    ruleData.params[paramMeta.name] = { type: 'AndRule', params: { rules: [] } };
                    updateUI();
                };
                paramDiv.appendChild(addBtn);
            } else {
                const wrap = document.createElement('div');
                wrap.appendChild(renderRule(val, ruleData, paramMeta.name));
                paramDiv.appendChild(wrap);
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
    rootContainer.appendChild(renderRule(window.ruleset));
    saveRuleset(window.ruleset);
    // Optionally auto-compile on change
    // compileRuleset();
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
        if (p.type === 'rules') {
            args.push((val || []).map(child => instantiateRule(child)));
        } else if (p.type === 'rule' || p.type === 'amount_rule') {
            args.push(instantiateRule(val));
        } else if (p.type === 'rule_optional') {
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
        document.getElementById('sql-output').textContent = result.sql;
        document.getElementById('params-output').textContent = JSON.stringify(result.params, null, 2);
    } catch (e) {
        console.error(e);
        document.getElementById('sql-output').textContent = "Compilation Error: " + e.message;
        document.getElementById('params-output').textContent = "";
    }
}

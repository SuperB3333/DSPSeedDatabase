function call_api() {
    const params = JSON.parse(document.getElementById('params-output').textContent)
    const data = {
        "query": document.getElementById('sql-output').textContent,
        "params": params
    }
    fetch("/start_query", {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data)
    })
        .then(result => {
            setStatus("Awaiting results...", "#CDAD00")
            status_loop(result.text())
        })
        .catch(error => {
            setStatus("Error while calling API: " + error, "#FF0000")
        })

}



function setStatus(text, col) {
    const element = document.getElementById("status-str")

    element.textContent = text
    element.style = "color: " + col
}

function status_loop(qid) {
    fetch("/query_ready", {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: '{"qid": "' + qid + '" }'
    })
        .then(response => response.text())
        .then(body => body === "true")
        .then(is_ready => {
            if (is_ready) {
                retrieve_seeds(qid)
            }
            else {
                setTimeout(() => {
                    status_loop(qid)
                }, 5_000);
            }
        })
        .catch(error => {
            setStatus("Error while checking status: " + error, "#FF0000")
        })
}

function retrieve_seeds(qid) {
    fetch("/get_result", {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: '{"qid": "' + qid + '" }'
    })
        .then(response => response.json())
        .then(seed_list => {
            let field = document.getElementById("results-seeds")
            if (seed_list.length === 0) {
                setStatus("No seeds found with the matching criteria", "#CDAD00")
            }
            else {
                field.hidden = false
                field.textContent = seed_list.join(", ")

                setStatus("Found Seeds", "#00FF00")
            }
        })
        .catch(error => {
            setStatus("Error while retrieving seeds: " + error, "#FF0000")
        })
}
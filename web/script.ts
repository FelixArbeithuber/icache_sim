import { EditorView, basicSetup } from "codemirror";
import init, { run_simulation } from "./lru_sim/lru_sim.js";

const editor = new EditorView({
    doc: "",
    parent: document.querySelector("#editor")!,
    extensions: [basicSetup],
});

const output = new EditorView({
    doc: "click 'Run' to simulate",
    parent: document.querySelector("#output")!,
    extensions: [EditorView.editable.of(false)],
});

function setText(cm: EditorView, text: string) {
    cm.dispatch({
        changes: { from: 0, to: cm.state.doc.length, insert: text },
    });
}

async function setTrace(trace: string) {
    return fetch(`/lru_sim/traces/${trace}`)
        .then((response) => response.text())
        .then((trace) => setText(editor, trace));
}

init().then(() => {
    const simulateBtn: HTMLButtonElement = document.querySelector("#simulate-btn")!;
    simulateBtn.addEventListener("click", () => {
        setText(output, "running simulation ...");
        // delay wasm simulation to give js time to update the text before blocking the UI
        setTimeout(() => {
            const trace = editor.state.doc.toString();
            const result = run_simulation(trace);
            setText(output, result);
        }, 0);
    });

    const tracesSelect: HTMLSelectElement = document.querySelector("#traces-select")!;
    tracesSelect.addEventListener("change", (e) => {
        setTrace((e.target! as HTMLSelectElement).value);
    });
    setTrace(tracesSelect.value);
});

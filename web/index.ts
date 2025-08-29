import { basicSetup } from "codemirror";
import { EditorView, keymap } from "@codemirror/view";
import { indentWithTab } from "@codemirror/commands";
import init, { run_simulation } from "./icache_sim/icache_sim.js";

// import so bun includes wasm blob when bundling
import * as wasm from "./icache_sim/icache_sim_bg.wasm";
// use the import in some way so it doesn't get optimized away
((_) => { })(wasm);

const editor = new EditorView({
    doc: "",
    parent: document.querySelector("#editor")!,
    extensions: [basicSetup, keymap.of([indentWithTab])],
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
    return fetch(`/icache_sim/traces/${trace}`)
        .then((response) => response.text())
        .then((trace) => setText(editor, trace));
}

init().then(() => {
    const hitCyclesInput: HTMLInputElement = document.querySelector("#hit-cycles-input")!;
    const missCyclesInput: HTMLInputElement = document.querySelector("#miss-cycles-input")!;
    const logMemoryAccesses: HTMLInputElement = document.querySelector("#log-memory-accesses-input")!;
    const simulateBtn: HTMLButtonElement = document.querySelector("#simulate-btn")!;
    simulateBtn.addEventListener("click", () => {
        setText(output, "running simulation ...");
        // delay wasm simulation to give js time to update the text before blocking the UI
        setTimeout(() => {
            const trace = editor.state.doc.toString();
            const result = run_simulation(trace, parseInt(hitCyclesInput.value), parseInt(missCyclesInput.value), logMemoryAccesses.checked);
            setText(output, result);
        }, 0);
    });

    const tracesSelect: HTMLSelectElement = document.querySelector("#traces-select")!;
    tracesSelect.addEventListener("change", (e) => {
        setTrace((e.target! as HTMLSelectElement).value);
    });
    setTrace(tracesSelect.value);
});

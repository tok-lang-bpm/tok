// The tok playground: the editor, the diagnostics and the projections of the document,
// computed by the tok engine compiled to WebAssembly (pkg/tok_wasm.js, built by
// tools/playground.sh). Every engine function takes the text and a JSON object of options
// and returns {"ok": value} or {"error": {kind, message}} (docs/c-api.md).

const $ = (id) => document.getElementById(id);

const state = {
  engine: null,
  generation: 0,
  lang: "en",
  bpmn: null,       // BPMN XML of the last document without errors
  viewer: null,     // the bpmn-js viewer, created on first use
  viewerXml: null,  // the XML the viewer shows
  timer: 0,
};

// ---------------------------------------------------------------- the engine

async function loadEngine() {
  // a fresh module after a trap: the old instance may be inconsistent
  const suffix = state.generation ? `?generation=${state.generation}` : "";
  const module = await import(`./pkg/tok_wasm.js${suffix}`);
  await module.default();
  state.engine = module;
  state.generation += 1;
}

// Call an engine function; returns {ok} or {error}. The functions whose names are reserved
// words of JavaScript (export, import, eval) are exported with an underscore: _export, …
function call(name, input, options = {}) {
  const fn = state.engine[name] ?? state.engine["_" + name];
  try {
    return JSON.parse(fn(input, JSON.stringify(options)));
  } catch (e) {
    loadEngine();
    return { error: { kind: "internal", message: `the engine stopped: ${e.message}; it has been restarted` } };
  }
}

// ---------------------------------------------------------------- the editor

const source = $("source");
const gutter = $("gutter");

function updateGutter() {
  const lines = source.value.split("\n").length;
  let text = "";
  for (let i = 1; i <= lines; i++) text += i + "\n";
  gutter.textContent = text;
  gutter.scrollTop = source.scrollTop;
}

function goToLine(line) {
  const lines = source.value.split("\n");
  let start = 0;
  for (let i = 0; i < line - 1 && i < lines.length; i++) start += lines[i].length + 1;
  const end = start + (lines[line - 1] || "").length;
  source.focus();
  source.setSelectionRange(start, end);
  const lineHeight = parseFloat(getComputedStyle(source).lineHeight) || 20;
  source.scrollTop = Math.max(0, (line - 4) * lineHeight);
}

function setText(text) {
  source.value = text;
  updateGutter();
  schedule(0);
}

source.addEventListener("input", () => { updateGutter(); schedule(); });
source.addEventListener("scroll", () => { gutter.scrollTop = source.scrollTop; });
source.addEventListener("keydown", (e) => {
  // Tab inserts two spaces: tok indents with spaces
  if (e.key === "Tab" && !e.shiftKey && !e.ctrlKey && !e.altKey) {
    e.preventDefault();
    const { selectionStart: s, selectionEnd: t } = source;
    source.setRangeText("  ", s, t, "end");
    updateGutter();
    schedule();
  }
});

function schedule(delay = 250) {
  clearTimeout(state.timer);
  state.timer = setTimeout(refresh, delay);
}

// ---------------------------------------------------------------- live analysis

function refresh() {
  if (!state.engine) return;
  const text = source.value;
  const checked = call("check", text, { lang: state.lang });
  const list = $("diagnostics");
  list.textContent = "";
  if (checked.error) {
    $("summary").textContent = checked.error.message;
    return;
  }
  const { errors, bpmn_class: cls, diagnostics } = checked.ok;
  const warnings = diagnostics.filter((d) => d.severity === "warning").length;
  const notes = diagnostics.filter((d) => d.severity === "note").length;
  $("summary").textContent = errors === 0 && diagnostics.length === 0
    ? `No diagnostics · BPMN conformance class: ${cls}`
    : `${errors} error(s), ${warnings} warning(s), ${notes} note(s)` + (errors === 0 ? ` · BPMN conformance class: ${cls}` : "");
  for (const d of diagnostics) {
    const li = document.createElement("li");
    li.className = `sev-${d.severity}`;
    li.textContent = `${d.line}: ${d.severity} ${d.code}: ${d.message}`;
    li.addEventListener("click", () => goToLine(d.line));
    list.append(li);
  }
  updateDecisions(text);
  const diagram = $("diagram");
  if (errors > 0) {
    // keep the last picture, dimmed, while the document has errors
    diagram.classList.add("stale");
    if (!diagram.firstChild) diagram.textContent = "The diagram appears when the document has no errors.";
    state.bpmn = null;
    return;
  }
  const svg = call("svg", text, { lang: state.lang, title: "model.svg" });
  if (svg.ok) {
    diagram.innerHTML = svg.ok;
    diagram.classList.remove("stale");
  } else {
    diagram.classList.add("stale");
  }
  const bpmn = call("bpmn", text, { lang: state.lang });
  state.bpmn = bpmn.ok || null;
  if (!$("panel-viewer").hidden) showViewer();
}

// ---------------------------------------------------------------- bpmn-js viewer

async function showViewer() {
  const container = $("viewer");
  if (typeof window.BpmnJS === "undefined") {
    container.textContent = "The bpmn-js viewer could not be loaded (it comes from unpkg.com).";
    return;
  }
  if (!state.bpmn) {
    if (!state.viewer) container.textContent = "The document has errors or nothing to draw.";
    return;
  }
  if (!state.viewer) {
    container.textContent = "";
    state.viewer = new window.BpmnJS({ container });
  }
  if (state.viewerXml === state.bpmn) return;
  state.viewerXml = state.bpmn;
  try {
    await state.viewer.importXML(state.bpmn);
    state.viewer.get("canvas").zoom("fit-viewport");
  } catch (e) {
    container.prepend(document.createTextNode(`bpmn-js: ${e.message}`));
  }
}

// ---------------------------------------------------------------- tabs

for (const tab of document.querySelectorAll("[role=tab]")) {
  tab.addEventListener("click", () => {
    for (const t of document.querySelectorAll("[role=tab]")) {
      const selected = t === tab;
      t.setAttribute("aria-selected", String(selected));
      $(`panel-${t.dataset.tab}`).hidden = !selected;
    }
    if (tab.dataset.tab === "viewer") showViewer();
    localStore("tab", tab.dataset.tab);
  });
}

// ---------------------------------------------------------------- actions

function download(name, text, type = "text/plain") {
  const url = URL.createObjectURL(new Blob([text], { type }));
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

function stem() {
  const m = source.value.match(/^\s*tok\s+\S+\s+"([^"]*)"/m);
  const title = m ? m[1] : "model";
  const ascii = title.normalize("NFD").replace(/[̀-ͯ]/g, "").replace(/ł/g, "l").replace(/Ł/g, "L");
  return ascii.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "model";
}

$("format").addEventListener("click", () => {
  const r = call("fmt", source.value, { lang: state.lang });
  if (r.ok !== undefined) setText(r.ok);
  else $("summary").textContent = r.error.message.split("\n")[0];
});

$("save-tok").addEventListener("click", () => download(`${stem()}.tok`, source.value));

$("save-bpmn").addEventListener("click", () => {
  const r = call("bpmn", source.value, { lang: state.lang });
  if (r.ok) download(`${stem()}.bpmn`, r.ok, "application/xml");
  else $("summary").textContent = r.error.message.split("\n")[0];
});

$("open").addEventListener("change", async (e) => {
  const file = e.target.files[0];
  if (!file) return;
  const text = await file.text();
  e.target.value = "";
  if (/\.tok$/i.test(file.name)) {
    setText(text);
    return;
  }
  const r = call("import", text, { layout: false });
  if (r.ok) {
    const warnings = r.ok.warnings.length ? `# not kept by the import:\n${r.ok.warnings.map((w) => `# ${w}`).join("\n")}\n` : "";
    setText(warnings + r.ok.text);
  } else {
    $("summary").textContent = `${file.name}: ${r.error.message}`;
  }
});

$("share").addEventListener("click", async () => {
  const bytes = new TextEncoder().encode(source.value);
  let binary = "";
  for (const b of bytes) binary += String.fromCharCode(b);
  const encoded = btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
  history.replaceState(null, "", `#doc=${encoded}`);
  try {
    await navigator.clipboard.writeText(location.href);
    $("summary").textContent = "The link to this document is in the clipboard.";
  } catch {
    $("summary").textContent = "The link to this document is in the address bar.";
  }
});

$("lang").addEventListener("change", (e) => {
  state.lang = e.target.value;
  localStore("lang", state.lang);
  schedule(0);
});

$("sim-form").addEventListener("submit", (e) => {
  e.preventDefault();
  const options = { lang: state.lang, log: false, cases_table: false };
  const cases = $("sim-cases").value;
  const seed = $("sim-seed").value;
  if (cases) options.cases = Number(cases);
  if (seed) options.seed = Number(seed);
  const r = call("sim", source.value, options);
  $("sim-report").textContent = r.ok ? r.ok.report : r.error.message;
});

function updateDecisions(text) {
  const select = $("eval-decision");
  const names = [...text.matchAll(/^\s*decision\s+([A-Za-z_][A-Za-z0-9_-]*)/gm)].map((m) => m[1]);
  const current = select.value;
  if ([...select.options].map((o) => o.value).join(" ") === names.join(" ")) return;
  select.textContent = "";
  for (const n of names) select.append(new Option(n, n));
  if (names.includes(current)) select.value = current;
}

$("eval-form").addEventListener("submit", (e) => {
  e.preventDefault();
  let values;
  try {
    values = JSON.parse($("eval-values").value || "{}");
  } catch (err) {
    $("eval-result").textContent = `The input values are not valid JSON: ${err.message}`;
    return;
  }
  const decision = $("eval-decision").value;
  if (!decision) {
    $("eval-result").textContent = "The document has no decision.";
    return;
  }
  const r = call("eval", source.value, { lang: state.lang, decision, values });
  $("eval-result").textContent = r.ok !== undefined ? JSON.stringify(r.ok, null, 2) : r.error.message;
});

$("export").addEventListener("click", () => {
  const list = $("files");
  list.textContent = "";
  const r = call("export", source.value, { lang: state.lang, name: stem() });
  if (r.error) {
    const li = document.createElement("li");
    li.textContent = r.error.message;
    list.append(li);
    return;
  }
  for (const [name, content] of Object.entries(r.ok)) {
    const li = document.createElement("li");
    const a = document.createElement("a");
    a.href = "#";
    a.textContent = name;
    a.addEventListener("click", (ev) => { ev.preventDefault(); download(name, content); });
    li.append(a, document.createTextNode(` · ${content.length.toLocaleString()} characters`));
    list.append(li);
  }
});

// ---------------------------------------------------------------- start

function localStore(key, value) {
  try {
    if (value === undefined) return localStorage.getItem(`tok-playground-${key}`);
    localStorage.setItem(`tok-playground-${key}`, value);
  } catch {
    return null;
  }
  return null;
}

function fromHash() {
  const m = location.hash.match(/^#doc=([A-Za-z0-9_-]+)$/);
  if (!m) return null;
  try {
    const binary = atob(m[1].replace(/-/g, "+").replace(/_/g, "/"));
    return new TextDecoder().decode(Uint8Array.from(binary, (c) => c.charCodeAt(0)));
  } catch {
    return null;
  }
}

async function loadExamples() {
  const select = $("example");
  try {
    const names = await (await fetch("examples/index.json")).json();
    for (const n of names) select.append(new Option(n.replace(/\.tok$/, ""), n));
  } catch {
    select.disabled = true;
  }
  select.addEventListener("change", async () => {
    if (!select.value) return;
    const text = await (await fetch(`examples/${select.value}`)).text();
    history.replaceState(null, "", location.pathname + location.search);
    setText(text.replace(/\r\n/g, "\n"));
  });
  return select.options.length ? select.options[0].value : null;
}

async function main() {
  state.lang = localStore("lang") || "en";
  $("lang").value = state.lang;
  // `?tab=viewer` opens a tab (links in the documentation), else the last one used
  const tab = new URLSearchParams(location.search).get("tab") || localStore("tab");
  if (tab) document.querySelector(`[data-tab="${tab}"]`)?.click();
  const first = await loadExamples();
  try {
    await loadEngine();
  } catch (e) {
    $("summary").textContent = `The engine could not be loaded: ${e.message}`;
    return;
  }
  $("version").textContent = `engine ${state.engine.version()}`;
  const shared = fromHash();
  if (shared !== null) {
    const select = $("example");
    select.prepend(new Option("(shared document)", ""));
    select.value = "";
    setText(shared);
  } else if (first) {
    const text = await (await fetch(`examples/${first}`)).text();
    setText(text.replace(/\r\n/g, "\n"));
  } else {
    setText('tok 1.0 "Nowy proces"\n\nstart s "Początek"\ntask a "Zadanie"\nend e "Koniec"\n\nflow s -> a -> e\n');
  }
}

main();

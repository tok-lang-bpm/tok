# Playground

The playground is a web page where a tok model is written and checked in the browser. The engine itself runs in the page, compiled to WebAssembly from the same Rust crates as the `tok` command, so the diagnostics, the canonical form, the diagram, the BPMN file, the simulation and the decisions are exactly those of `tok`. Nothing is sent to a server: the page is a set of static files that any static host can serve.

## What the page does

- **Editor** with line numbers. Every change is checked after a short pause; the diagnostics appear below the editor with their line, severity, code and message, in English or Polish (*Messages*), and a click on one selects its line. The summary gives the numbers of errors, warnings and notes and the smallest BPMN conformance class of the document.
- **Diagram**: the SVG picture of the model, redrawn whenever the document has no errors (while it has errors, the last picture stays, dimmed).
- **bpmn-js**: the BPMN 2.0 XML of the model shown in the bpmn-js viewer of bpmn.io, which can be zoomed and moved; this is how the file looks in bpmn.io and Camunda Modeler.
- **Simulation**: runs `tok sim` with the cases and seed of the model, or the ones given, and shows the report.
- **Decisions**: evaluates a decision of the document for input values written as a JSON object, as `tok eval`.
- **Files**: every file `tok export` writes (BPMN, SVG, DMN with its pictures, Camunda forms), each one a download.
- **Format** replaces the text with its canonical form; **Download .tok** and **Download BPMN** save the document and its BPMN file; **Open…** loads a `.tok` file, or imports a BPMN or DMN file into tok; **Example** loads the examples of the repository.
- **Copy link** puts the document into the address (`#doc=…`, the text in base64url) and copies the link, so a model can be shared without a server. `?tab=viewer` (or `diagram`, `simulation`, `decisions`, `files`) opens a tab.

## Building

The build needs the Rust target `wasm32-unknown-unknown` and the `wasm-bindgen` command of the exact version of the `wasm-bindgen` crate of `rust/tok-wasm` (0.2.129):

```
rustup target add wasm32-unknown-unknown            # in rust/, for the toolchain of the repository
cargo install --locked wasm-bindgen-cli --version 0.2.129
bash tools/playground.sh                              # into rust/target/playground
bash tools/playground.sh -o site/                     # into another folder
bash tools/playground.sh --test                       # also the tests of the module under Node.js
```

The script installs the `wasm-bindgen` command itself when it is missing or of another version. It compiles the crate `tok-wasm` in release mode, writes the WebAssembly module and its JavaScript glue (`pkg/tok_wasm_bg.wasm`, `pkg/tok_wasm.js`) with `wasm-bindgen --target web`, and copies the page (`playground/index.html`, `app.js`, `style.css`), the licence and the examples with their list (`examples/index.json`). `tools/check.sh` builds the playground whenever the target and the command are installed, and runs the tests of the module when Node.js is installed.

To try it locally, serve the folder over HTTP (browsers load WebAssembly modules only over HTTP, not from `file:` addresses):

```
python -m http.server -d rust/target/playground 8000
```

and open `http://localhost:8000/`.

## Publishing

The folder is a complete static site with relative links, so it works at the root of a domain or in a subfolder. To publish it on Codeberg Pages, copy the contents of the built folder to the branch `pages` of the repository (or to a repository named `pages`) and push it; the site appears at `https://USER.codeberg.page/REPOSITORY/`. Any other static host (GitHub Pages, a web server, object storage with web hosting) takes the same files. The server must send `.wasm` files as `application/wasm`, which all common hosts do; the module is about 3.7 MB and compresses to about a third with the gzip or Brotli compression hosts apply.

## The engine in a page

`pkg/tok_wasm.js` is an ES module. Its default export loads the WebAssembly module; the named exports are the functions of the engine, which take the text of a document (or of an XML file) and the options as a JSON string and return a JSON string, `{"ok": VALUE}` or `{"error": {"kind", "message"}}`, exactly as the C interface does (`c-api.md` documents every option and result):

The names are `check`, `fmt`, `bpmn`, `svg`, `export`, `import`, `eval`, `sim`, `version` and `cheatsheet`. `export`, `import` and `eval` cannot be the names of variables in JavaScript, so a page renames them when it imports them (`import init, { check, export as exportFiles, import as importFile, eval as evaluate } from "./pkg/tok_wasm.js"`) or uses the module as a namespace:

```js
import init, * as tok from "./pkg/tok_wasm.js";

await init();
const r = JSON.parse(tok.check(text, JSON.stringify({ lang: "pl" })));
if (r.ok) console.log(r.ok.errors, r.ok.diagnostics);
const picture = JSON.parse(tok.svg(text, "{}")).ok;
```

`app.js` uses the namespace; its function `call` shows how the page parses the results and recovers from an internal error.

## The bpmn-js viewer

The *bpmn-js* tab uses the viewer of bpmn-js 18.31.0 by bpmn.io (Camunda Services GmbH), loaded from unpkg.com with Subresource Integrity hashes, so the browser refuses any other file. Its licence (https://unpkg.com/bpmn-js@18.31.0/LICENSE) permits use and redistribution and asks that the bpmn.io watermark, which the viewer draws in the corner of the diagram, stays visible. Without network access to unpkg.com the tab says the viewer could not be loaded; everything else works offline once the page is loaded. To serve the page without the content delivery network, place the three files (`bpmn-viewer.production.min.js`, `diagram-js.css`, `bpmn-js.css`, with the licence) next to `index.html` and change the three addresses.

## Tests

- `cargo test -p tok-wasm` (part of `cargo test --workspace`) calls every function of the crate on the host and compares the results with those of the C interface.
- `tools/playground.sh --test` runs the tests in `rust/tok-wasm/tests/web.rs` inside the WebAssembly module under Node.js with `wasm-bindgen-test-runner`: check, format, draw, export, import, simulate (on the stack of the module, see below) and evaluate a decision of language 0.7, including `today()`.
- The page was checked in a headless browser (Microsoft Edge): it loads the engine and an example, draws the diagram, shows the BPMN file in bpmn-js, and lists the diagnostics of a document with errors.

## Limits

- WebAssembly has no threads here, so the simulator runs on the stack of the module, which the workspace sets to 32 MiB (`rust/.cargo/config.toml`); the `tok` command gives it a thread with 256 MiB. A model whose cases pass through very long chains of gateways and events without time in between could exhaust the smaller stack.
- A browser has no system clock for Rust; FEEL `now()` and `today()` read the clock of the page (`Date.now()`), in UTC.
- If the engine stops on an internal error (a panic, which aborts a WebAssembly module), the page reports it, writes the message to the console of the browser and loads a fresh module.
- The page has no layout sidecar: imported BPMN files are drawn with the automatic layout.

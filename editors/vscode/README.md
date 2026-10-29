# tok for Visual Studio Code

Support for the [tok](https://github.com/tok-lang-bpm/tok) process language: plain-text models of BPMN 2.0 processes with their DMN decisions, Camunda forms and simulation parameters, in `.tok` files.

- **Syntax highlighting** of statements, identifiers, labels, attributes, flags, flows, comments and decision tables.
- **Diagnostics** as you type: every check of `tok check` — the structure of the document, the BPMN 2.0 rules, the Camunda 8 profile of executable models, decisions, FEEL and simulation parameters — in English or Polish.
- **Formatting** to the canonical form of `tok fmt` (*Format Document*, or format on save).
- **Go to definition, find references, highlight and rename** of identifiers: pools, lanes, flow nodes, data, decisions, input data, forms, resources and every other element, including the names of a decision model in its FEEL expressions.
- **Outline** of pools, lanes, nodes, decisions, forms and the other blocks; **hover** texts for elements, keywords and diagnostic codes; **completion** of keywords, identifiers and attributes.
- **tok: Show diagram** opens the diagram of the model beside the editor and redraws it as you edit.
- **tok: Export BPMN** writes `NAME.bpmn` (BPMN 2.0 XML with diagram layout, for Camunda Modeler and bpmn.io) next to the document; **tok: Export all files** writes every file of `tok export` (BPMN, SVG, DMN, forms) into a folder you choose; **tok: Check** summarizes the diagnostics.

The work is done by `tok-lsp`, the language server of tok (`docs/lsp.md` of the repository), which runs the tok engine itself, so the editor gives the same results as the `tok` command.

## Requirements

The packages for Windows, Linux and macOS carry `tok-lsp` for their platform; nothing else is needed. With a package without the server, or to use another build, install `tok-lsp` (see *Building* below) and put it on the `PATH`, or set **tok.server.path** to the program.

## Settings

| Setting | Default | Meaning |
|---|---|---|
| `tok.server.path` | empty | path of `tok-lsp`; empty: the server of the extension, else `tok-lsp` on the `PATH` |
| `tok.lang` | `auto` | language of the diagnostics: `en`, `pl`, or `auto` (Polish when VS Code runs in Polish) |
| `tok.level` | `full` | teaching level, the elements a document may use: `basic`, `analytic`, `full` (as `tok check --level`) |
| `tok.class` | `none` | BPMN 2.0.2 conformance class to check against: `descriptive`, `analytic`, `common-executable`, `full` (as `tok check --class`) |

Changes of `tok.lang`, `tok.level` and `tok.class` apply at once; a change of `tok.server.path` restarts the server (also *tok: Restart language server*).

## The diagram

*tok: Show diagram* (also the preview button in the title bar of a `.tok` editor) shows the SVG picture that `tok svg` draws, from the text in the editor, saved or not. While the document has errors the panel shows them instead. When a layout sidecar `FILE.tok.layout` lies next to the file (written by `tok import`), the diagram keeps the drawing of the imported model.

## Building

From the repository, with Rust 1.99 or newer and Node.js 22 or newer:

```
cd editors/vscode
npm ci
npm test               # checks the types, bundles the extension, runs the grammar tests
npm run package        # builds tok-lsp if needed and writes tok-PLATFORM-VERSION.vsix
```

`npm run build` checks the TypeScript sources with the compiler and bundles them with the language client into one file, `out/extension.js` (esbuild), so the package holds no `node_modules`. `npm run package` copies `rust/target/release/tok-lsp` (built with `cargo build --release -p tok-lsp` when it is missing) into the package, so the `.vsix` is for the platform it is built on (`win32-x64`, `linux-x64`, `darwin-arm64`, …). Install it with *Extensions: Install from VSIX…* or `code --install-extension tok-win32-x64-1.0.0.vsix`. `tools/check.sh` of the repository builds, tests and packages the extension when npm is available and copies the `.vsix` to `dist/`.

## Tests

- `npm run test:grammar`: the grammar tests in `test/grammar/*.tok` (vscode-tmgrammar-test), which assert the scope of every construct.
- `npm run test:examples`: the grammar over every example and conformance document of the repository; statement keywords, comments and table rows must get their scopes, and no string may run over its line.
- `npm run smoke` (after `npm run package`): unpacks the `.vsix`, checks its manifest, the one bundled code file (and that no `node_modules` are packaged), grammar and server, drives the bundled `tok-lsp` through the protocol (diagnostics in Polish, a fix, the diagram request, shutdown), and when the `code` command is available installs the package into an empty extensions folder of VS Code and checks that VS Code lists it. It needs no display.
- `npm run test:vscode`: runs `test/vscode/index.js` inside VS Code (a window opens, so it needs a desktop): the extension starts the server, the diagnostics appear and disappear after a fix, the document is formatted, the outline and go to definition work, *tok: Show diagram* opens its panel and *tok: Export BPMN* writes the file. The `code` command is found on the `PATH` or in the variable `VSCODE_CLI`.

## Installing

Install **tok** (identifier `tok-lang-bpm.tok`) from the Extensions view of VS Code, from the [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=tok-lang-bpm.tok), or, in VSCodium, Eclipse Theia and other editors that use it, from [Open VSX](https://open-vsx.org/extension/tok-lang-bpm/tok). The platform packages (Windows x64, Linux x64, macOS arm64) are also attached to every release of https://github.com/tok-lang-bpm/tok/releases; install one with `code --install-extension tok-win32-x64-1.0.1.vsix`.

A universal package without a server (`npx vsce package` with an empty `server/` folder) works on every platform that has `tok-lsp` on the `PATH`.

## Licence

GPL-3.0-or-later, as tok.

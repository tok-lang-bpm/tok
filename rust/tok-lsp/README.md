# tok-lsp

The language server of the tok process language (LSP 3.17 over standard input and output):
diagnostics of every layer of the checker in English or Polish, formatting to the canonical
form, go to definition, find references, highlight and rename of identifiers (also the FEEL
names of decision models), outline, hover, completion, and requests that return the SVG
diagram, the BPMN file and every exported file of a document.

```
cargo install tok-lsp
```

Configuration for VS Code (the extension in `editors/vscode` bundles the server), Neovim,
Helix and Emacs is in `docs/lsp.md` of the repository.

License: GPL-3.0-or-later.

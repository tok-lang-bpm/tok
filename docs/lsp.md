# Language server

`tok-lsp` is the language server of tok: a program that an editor starts and talks to over standard input and output with the Language Server Protocol (LSP 3.17). It analyses the open documents with the engine itself, so the editor shows exactly the diagnostics of `tok check` and formats exactly as `tok fmt`, and it adds what an editor needs on top: navigation between the identifiers of a model, rename, an outline, hover texts, completion and the diagram of the model. The VS Code extension in `editors/vscode` bundles it; any other editor with an LSP client can use it (see [Other editors](#other-editors)).

## Installing

Install it from crates.io, or build it from the repository (Rust 1.99 or newer):

```
cargo install tok-lsp
cd rust && cargo build --release -p tok-lsp      # target/release/tok-lsp (tok-lsp.exe on Windows)
```

and put the program on the `PATH`, or give its full path to the editor. `tools/check.sh` builds it and copies it to `dist/` together with `tok`. `tok-lsp --version` prints the version; without arguments (or with `--stdio`, which editors often add) it serves one editor on its standard streams and ends when the editor sends `exit`.

## What it does

### Diagnostics

The editor sends the text of a document with `textDocument/didOpen`, every change as the whole new text with `textDocument/didChange` (full synchronization) and the end of editing with `textDocument/didClose`. When a document is opened and after every change the server checks it completely, as `tok check` does, and sends the diagnostics with `textDocument/publishDiagnostics`: tokens and lines, statements, references and structure, the BPMN 2.0 rules of the exported model (`E`, `W`, `L` codes), the executable profile for Camunda 8, decisions, FEEL and simulation parameters (spec §13). The language version of the header decides what the document may contain, as it does for the command line, so a 0.5 document is checked as 0.5, a 0.7 document as 0.7 and a 1.0 document as 1.0 (with the deprecation warnings of 1.0, `R090`).

Each diagnostic carries its code (`R002`, `E008`, …), its severity (error, warning, or information for the style notes) and its message. It points at the identifier it is about when that identifier stands on its line, otherwise at the whole statement. Messages are in English, or in Polish when the editor runs in Polish (the `locale` of the editor is `pl`) or the setting `lang` is `pl`.

Two settings restrict what a document may contain, as the options of `tok check` do:

| Setting | Values | Meaning |
|---|---|---|
| `lang` | `en`, `pl` | language of the messages |
| `level` | `basic`, `analytic`, `full` (default) | teaching level (`tok check --level`) |
| `class` | `descriptive`, `analytic`, `common-executable`, `full`, or `null` (default) | BPMN 2.0.2 conformance class (`tok check --class`, code `R075`) |

The editor sends them as `initializationOptions` when it starts the server (`{"lang": "pl", "level": "analytic"}`) and may change them later with `workspace/didChangeConfiguration`, either as `{"settings": {"tok": {…}}}` or directly as `{"settings": {…}}`; every open document is checked again. Values that are not valid are ignored.

### Formatting

`textDocument/formatting` returns the canonical form of the document (spec §14), the text `tok fmt` prints, as one edit of the whole document; a document already in canonical form needs no edit. A document with Windows line endings keeps them. A document with syntax errors (any error of the lexer or the parser, such as an unterminated string or a duplicate identifier) is not formatted: the request fails with a message that names the first error, because the canonical form of a document that does not parse would lose text.

### Identifiers

Identifiers are the names of the elements of a model (spec §2): flow nodes, pools, lanes, data, decisions, input data, business knowledge models, knowledge sources, forms, resources, item definitions, global tasks, interfaces and operations, correlation keys, conversation nodes, choreographies and their activities, and flows and message flows with `id=`. The server knows where each one is declared and every place that refers to it:

- the node names of `flow` and `message` lines (`flow a -> b -> c`, `message a -> pool`);
- the activity and the data of `reads` and `writes`;
- the values of the attributes that name elements: `on=`, `process=`, `task=`, `form=`, `decision=`, `by=`, `operation=`, `compensate=`, `activity=`, `from=`, `to=`, `choreography=`, `collaboration=`, `input=`, `output=`, `invoke=`, `type=` (a DMN type or item definition), and the lists `members=`, `pools=`, `messages=`, `requires=` and the pairs of `map=`;
- in a document of language 0.7 or 1.0, the FEEL names of the decision requirements graph in the expressions of decisions and business knowledge models (`expr=`, the right-hand sides of `with=`, `default=`, the input expressions in the header of a decision table and the cells of its rules), and the DMN types named as the types of table columns (`| konto: konto-t || … |`). The FEEL name of an element is its identifier unless `name=` gives another one; a decision sees only what it `requires`, so a parameter or a context entry with the same name as an element is not a reference to it. FEEL reads the longest name of the document, so a word at the start of a longer name (`kara` in an entry `kara za tydzień`) is not a reference, and neither is the key of a context (`{kara: …}`). Names inside FEEL string literals are not references.

With this the server answers:

- **Go to definition** (`textDocument/definition`) from any reference, or from the declaration itself;
- **Find references** (`textDocument/references`), with or without the declaration;
- **Highlight** (`textDocument/documentHighlight`): the declaration and the references of the identifier under the cursor;
- **Rename** (`textDocument/prepareRename`, `textDocument/rename`): every occurrence is replaced. The new name must be an identifier (`[A-Za-z_][A-Za-z0-9_-]*`, ASCII, since identifiers become XML `id` attributes) and must not be used by another element; otherwise the rename fails with the reason (`identifier 'gotowe' is already used on line 10`). The derived identifiers of flows and message flows (`f-SOURCE-TARGET`, `m-SOURCE-TARGET`, spec §6.1) follow the names of their ends, also where another statement names them (`messages="m-a-b"`). A derived identifier itself is not written in the document and cannot be renamed; give the flow an `id=` to name it.

### Outline

`textDocument/documentSymbol` gives the statements that declare something, nested as their blocks: pools with their lanes and flow nodes, sub-processes with their content, choreographies, decisions with their context entries, forms with their fields, interfaces with their operations, keys with their properties, DMN types with their components, data, resources and simulation variables. Each symbol shows its kind and label.

### Hover

`textDocument/hover`: over an identifier the server shows the kind of the element, its label, the element of BPMN or DMN it exports and the line of its declaration; over the keyword of a statement, what the statement declares; over a diagnostic, its code with the layer it belongs to, its message, and the English and Polish templates of the code; over a diagnostic code written anywhere (in a comment, for example), the same explanation.

### Completion

`textDocument/completion`: the server offers the keywords at the start of a statement, the identifiers of the right kind after the keys that name elements (activities after `on=`, forms after `form=`, decisions after `decision=`, resources after `by=`, pools after `from=` and `to=`, business knowledge models after `invoke=`, …), the hit policies after `hit=` and the field types after `type=` on a form field, the flow nodes after `->` on a `flow` line (and pools on a `message` line), the activities and data of `reads` and `writes`, and the attribute keys and flags of a statement after its identifier.

### Positions

Columns are counted in UTF-16 code units, the encoding every LSP client supports, so labels in Polish or with characters outside the Basic Multilingual Plane do not shift the positions of what follows them. Documents are synchronized in full on every change.

## Requests for editor extensions

Besides the standard methods the server answers four requests that an editor extension can use to show and export a model without running the `tok` command. Their parameters are `{"textDocument": {"uri": URI}}` of an open document; they use the text the editor holds, not the file on disk.

| Method | Result | As |
|---|---|---|
| `tok/check` | `{"errors", "warnings", "notes", "bpmn_class"}`: the numbers of diagnostics of each severity and the smallest BPMN conformance class of the document | `tok check --json` |
| `tok/svg` | `{"svg": TEXT}`: the picture of the diagram | `tok svg` |
| `tok/bpmn` | `{"bpmn": TEXT}`: BPMN 2.0 XML with diagram layout | `tok bpmn` |
| `tok/export` | `{"files": {NAME: TEXT, …}}`: every file `tok export` writes (BPMN, SVG, DMN files with their pictures, forms); the optional parameter `"name"` is the stem of the BPMN and SVG names, by default the stem of the file name | `tok export` |

When the document is a file and the layout sidecar `FILE.tok.layout` exists next to it (spec §16), `tok/svg`, `tok/bpmn` and `tok/export` keep the geometry it holds, as the command line does. A document with errors, or one with nothing to draw, makes the request fail with the message of the engine (error code `-32803`, `RequestFailed`), which lists the errors. The results are those of the C interface (`c-api.md`), because the server calls the same functions.

## Other editors

Every editor with an LSP client can use `tok-lsp`: it needs the command (`tok-lsp`), the file type of `.tok` files and, optionally, the settings above as initialization options. Comments start with `#` and indentation is two spaces.

### Neovim

Neovim 0.11 and later configure language servers without plugins. In `init.lua`:

```lua
vim.filetype.add({ extension = { tok = "tok" } })

vim.lsp.config("tok", {
  cmd = { "tok-lsp" },
  filetypes = { "tok" },
  root_markers = { ".git" },
  init_options = { lang = "en", level = "full" },
})
vim.lsp.enable("tok")

vim.api.nvim_create_autocmd("FileType", {
  pattern = "tok",
  callback = function()
    vim.bo.commentstring = "# %s"
    vim.bo.expandtab = true
    vim.bo.shiftwidth = 2
  end,
})
```

Diagnostics appear as the file is edited; `vim.lsp.buf.format()` formats, `CTRL-]` goes to the definition (Neovim sets `tagfunc` to the server), `grr` lists the references, `grn` renames, `gO` shows the outline and `K` the hover text.

### Helix

In `languages.toml` (in the configuration folder of Helix, or `.helix/languages.toml` of a project):

```toml
[language-server.tok-lsp]
command = "tok-lsp"
config = { lang = "en", level = "full" }

[[language]]
name = "tok"
scope = "source.tok"
file-types = ["tok"]
comment-token = "#"
indent = { tab-width = 2, unit = "  " }
language-servers = ["tok-lsp"]
```

Helix sends `config` as the initialization options. `:format` formats the document, `gd` goes to the definition, `gr` lists the references, `<space>r` renames and `<space>s` opens the outline. `hx --health tok` shows whether Helix finds the server.

### Emacs (eglot)

Emacs 29 and later include eglot. A small major mode gives `.tok` files their comments and indentation, and eglot starts the server for it:

```elisp
(define-derived-mode tok-mode prog-mode "tok"
  "Major mode for process models in the tok language."
  (modify-syntax-entry ?# "<" tok-mode-syntax-table)
  (modify-syntax-entry ?\n ">" tok-mode-syntax-table)
  (modify-syntax-entry ?- "_" tok-mode-syntax-table)
  (setq-local comment-start "# ")
  (setq-local indent-tabs-mode nil)
  (setq-local tab-width 2))

(add-to-list 'auto-mode-alist '("\\.tok\\'" . tok-mode))

(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs
               '(tok-mode . ("tok-lsp" :initializationOptions (:lang "en" :level "full")))))

(add-hook 'tok-mode-hook #'eglot-ensure)
```

`M-x eglot-format-buffer` formats, `M-.` goes to the definition, `M-?` lists the references, `M-x eglot-rename` renames and `M-x imenu` uses the outline; Flymake shows the diagnostics.

### Other clients

Sublime Text (package LSP), Kate, Zed and the other editors with an LSP client take the same three things: the command `tok-lsp`, the file extension `.tok` and, optionally, the initialization options `{"lang", "level", "class"}`.

## Limits

- The server analyses each document on its own, as `tok check` does: identifiers are not resolved across files, and a `process=` of a call activity that names a process of another file is left alone.
- FEEL names are found in the text of the expressions, outside string literals, without parsing FEEL. An element with `name=` is referred to in FEEL by that name, so renaming its identifier changes `requires=` and the other attributes but not the expressions, which is what the document needs.
- The diagram of `tok/svg` is the automatic layout of the engine, or the sidecar geometry when the file has one; the server does not write files.

# MCP server

`tok-mcp` gives the tok engine to AI agents through the Model Context Protocol (MCP). An MCP client (an agent application, an editor with an agent, or any program that speaks MCP) starts it as a local program and calls its tools: check a model and read the diagnostics, print the canonical form, export BPMN, SVG, DMN and form files, import BPMN or DMN, simulate, evaluate decisions, and read the language reference. The agent works with the text of a model and never has to write XML or diagram coordinates.

The skill `skills/tok-modelling` teaches an agent the workflow of modelling in tok; it uses the MCP server when the client has it and the `tok` command otherwise.

## Installing

Build it from the repository (Rust 1.99 or newer):

```
cd rust
cargo build --release -p tok-mcp      # target/release/tok-mcp (tok-mcp.exe on Windows)
```

`tools/check.sh` builds it and copies it to `dist/`. Put the program on the `PATH` or give the client its full path. `tok-mcp --version` prints the version; without arguments it serves one client on its standard input and output (the stdio transport of MCP: one JSON-RPC message per line) until the input ends.

## Configuring a client

The server needs no arguments and no environment. Clients that read a JSON configuration with an `mcpServers` object take:

```json
{
  "mcpServers": {
    "tok": {
      "command": "tok-mcp"
    }
  }
}
```

With a full path on Windows: `"command": "C:\\tools\\tok\\tok-mcp.exe"`. VS Code reads MCP servers of a workspace from `.vscode/mcp.json`:

```json
{
  "servers": {
    "tok": {
      "type": "stdio",
      "command": "tok-mcp"
    }
  }
}
```

Clients configured on the command line take the command `tok-mcp` as the server of the stdio transport. Paths given to the tools (`path`) are read relative to the working directory of the server, which is the one the client starts it in; give absolute paths when in doubt. To try the server by hand, the MCP Inspector of the protocol project starts it and lists its tools: `npx @modelcontextprotocol/inspector tok-mcp`.

## Protocol

The server speaks both eras of MCP:

- **revision 2026-07-28**, the current one: there is no handshake; every request carries its protocol version and the client capabilities in `_meta` (`io.modelcontextprotocol/protocolVersion`, `io.modelcontextprotocol/clientCapabilities`), `server/discover` describes the server, and every result carries `"resultType": "complete"` and the server's identity in `_meta`. A request with a version the server does not support gets the error `-32022` with the supported versions; a request without the required metadata, `-32602`.
- **revisions 2025-11-25, 2025-06-18, 2025-03-26 and 2024-11-05**, which open a session with `initialize`: the server answers with the version the client asked for when it supports it, and with 2025-11-25 otherwise, and serves the session in that revision. Results of tools carry `structuredContent` from revision 2025-06-18.

Its capabilities are `tools` and `resources`. It keeps no state between calls: every call reads the document it is given.

## Tools

Every tool that reads a document takes it either as `text` (the tok text) or as `path` (a `.tok` file the server reads), never both; `import` takes `xml` or `path`. `lang` (`en` or `pl`) chooses the language of diagnostics and reports. The input schemas are exact: an argument that is not listed, has the wrong type or a value outside its choices is an error of the tool, reported before anything runs.

| Tool | Arguments | Result (`structuredContent`) |
|---|---|---|
| `check` | `text` or `path`, `lang`, `level` (`basic`, `analytic`, `full`), `class` (`descriptive`, `analytic`, `common-executable`, `full`), `bpmn` (boolean, default `true`: also the BPMN 2.0 rules) | `{"errors", "bpmn_class", "diagnostics": [{"code", "severity", "line", "id", "message"}]}`, as `tok check --json`; the text content lists the diagnostics one per line |
| `fmt` | `text` or `path`, `lang` | `{"text"}`: the canonical form, as `tok fmt` (the server does not write the file) |
| `export` | `text` or `path`, `formats` (a list of `bpmn`, `svg`, `dmn`, `forms`; all when left out), `name` (stem of the BPMN and SVG names; default the stem of `path`, or `model`), `sidecar` (boolean, default `true`), `lang` | `{"files": {NAME: CONTENT}}`: the files `tok export` writes — `NAME.bpmn` with diagram layout, `NAME.svg`, the DMN files of the decisions (`dmn` includes the `.drd.svg` pictures of language 0.7) and the Camunda forms `ID.form` |
| `import` | `xml` or `path`, `format` (`auto`, `bpmn`, `dmn`), `flow_ids` (boolean, default `true`), `layout` (boolean, default `true`) | `{"format", "text", "warnings", "layout"}`: the tok document, what the import did not keep, and for BPMN the layout sidecar to save as `FILE.tok.layout` |
| `sim` | `text` or `path`, `cases`, `seed`, `log`, `cases_table`, `csv`, `xes` (booleans, default `false`), `lang` | `{"kpi", "report"}` and, on request, `log`, `cases`, `log_csv`, `cases_csv`, `xes`, as the C interface gives them (`c-api.md`, Simulation) |
| `eval` | `text` or `path`, `decision` (required), `values` (an object input name → JSON value), `lang` | `{"value"}`: the value of the decision, as `tok eval` prints it |
| `reference` | `topic`: `cheatsheet` (default), `spec`, `diagnostics`, `feel`, `cli` | `{"text"}`: the document of the repository (`docs/cheatsheet.md`, `docs/spec.md`, …) |

With `path`, `export` uses the layout sidecar `FILE.tok.layout` next to the file when it exists (spec §16), as `tok export` does; `"sidecar": false` ignores it. `sim` leaves out the event log and the case table by default because they grow with the number of cases; ask for them when the analysis needs them.

Every tool also returns its result as text: the diagnostics as lines, the canonical text, the report of the simulation, or the JSON of the result.

### Errors

Two kinds of errors are kept apart, as MCP asks:

- **Tool errors** (`"isError": true` with a text explaining the error) are what the agent can fix: arguments that do not match the schema, a file that cannot be read, a document with errors (the text lists them and gives every diagnostic as JSON), nothing to draw, an unknown decision, a violated hit policy, a model the simulator cannot run. `check` is the exception: a document with errors is its normal result (`errors` > 0), not an error of the tool.
- **Protocol errors** (JSON-RPC errors) are for malformed requests: an unknown tool or method (`-32602`, `-32601`), invalid JSON (`-32700`), missing or unsupported protocol metadata (`-32602`, `-32022`).

## Resources

The language reference is also available as resources, for clients that attach resources to the context of the agent:

| URI | Content |
|---|---|
| `tok://reference/cheatsheet` | the language on one page |
| `tok://reference/spec` | the specification (normative) |
| `tok://reference/diagnostics` | every diagnostic code in English and Polish |
| `tok://reference/feel` | FEEL in tok |
| `tok://reference/cli` | the `tok` command line |

All are `text/markdown`; `resources/read` returns the text.

## A session

An agent asked to model a process typically reads `tok://reference/cheatsheet` (or calls `reference`), writes the model, and then:

1. calls `check` with the text, reads the diagnostics by line and code, fixes the text and checks again until `errors` is 0;
2. calls `fmt` and writes the canonical text to the `.tok` file;
3. calls `export` with the path of the file and writes the files it needs (`formats: ["bpmn", "svg"]` for a diagram);
4. for an executable or simulated model, calls `sim` with a seed and reports the indicators with the seed, or `eval` for the decisions.

## Security

The server runs with the rights of the user who starts it. It reads the files named by `path` and the layout sidecars next to them, and writes nothing; it opens no network connection. A client that lets an agent call tools should still show the calls to the user, as MCP recommends.

# C interface

The tok engine is also a C library, `tokengine`, built from the crate `rust/tok-capi`. It gives every function of the engine to any language that can call C: the Julia package `Tok.jl` uses it through `ccall` (see [julia.md](julia.md)), and C, C++, Go, Zig or Fortran programs can link it directly. The declarations are in the header `rust/tok-capi/include/tok.h`. The functions behind it (the module `tok_cli::api` of the crate `tok-cli`) also serve the MCP server ([mcp.md](mcp.md)), the requests of the language server ([lsp.md](lsp.md)) and the WebAssembly build of the playground ([playground.md](playground.md)), so all of them take the same options and give the same results.

## Building

```
cd rust
cargo build --release -p tok-capi
```

| Platform | Shared library | Static library |
|---|---|---|
| Linux | `target/release/libtokengine.so` | `target/release/libtokengine.a` |
| macOS | `target/release/libtokengine.dylib` | `target/release/libtokengine.a` |
| Windows | `target\release\tokengine.dll` (import library `tokengine.dll.lib`) | `target\release\tokengine.lib` |

`tools/check.sh` builds the shared library and copies it with `tok.h` to `dist/`. For Julia the library is also packaged as the binary package `tok_jll` (`julia/binarybuilder`).

A program links the shared library as usual, for example on Linux:

```
cc app.c -I rust/tok-capi/include -L rust/target/release -ltokengine -o app
```

The static library contains the Rust standard library; a program that links it statically also links the system libraries the standard library uses, which `cargo rustc --release -p tok-capi --crate-type staticlib -- --print native-static-libs` lists for the platform.

## Conventions

The interface is small and stable, and every function has the same shape:

```c
char *tok_check(const char *text, const char *options);
```

- **Arguments** are NUL-terminated UTF-8 strings: the text of a document (or of an XML file, an expression, a list of command-line arguments) and the **options**, a JSON object such as `{"lang": "pl"}`. `NULL` or an empty string means no options. An option that the function does not know is an error, so that misspelt options do not go unnoticed. Options left out take the default of the command line; an option set to `null` is left out.
- **The result** is a newly allocated NUL-terminated UTF-8 string holding one JSON object: `{"ok": VALUE}` on success, `{"error": {"kind": KIND, "message": TEXT}}` on failure. The caller frees it with `tok_free`, exactly once. No function returns `NULL`.
- **Errors** have a kind (below) and a message in English (diagnostics in the language of the `lang` option). An error of kind `document` also has `diagnostics`, the list of every diagnostic of the document, as `tok_check` gives them.
- **Threads.** The functions keep no state between calls, are reentrant and can be called from several threads at the same time.
- **Panics.** A Rust panic never unwinds into the caller: every function catches it and returns an error of kind `internal`.
- **Documents** are read with universal newlines (`\r\n` and `\r` become `\n`), as the `tok` program reads files.
- **Numbers** in results follow the JSON written by the `tok` program: integers, and floats with a decimal point or an exponent (non-finite floats as `NaN`, `Infinity`, `-Infinity`).

| Error kind | Meaning |
|---|---|
| `invalid_argument` | a `NULL` text, invalid UTF-8, options that are not a JSON object, an unknown option or an option of the wrong type |
| `document` | the document has errors; `diagnostics` lists them |
| `render` | nothing to draw (a document without elements), or a failure of the layout |
| `import` | a file that cannot be imported (not XML, not BPMN or DMN) |
| `unknown_decision` | `tok_eval` for a decision the document does not have |
| `decision` | a violated hit policy |
| `feel` | an expression that cannot be evaluated |
| `distribution` | an invalid distribution of the simulation statements |
| `simulation` | a model the simulator cannot run, or invalid simulation parameters |
| `io` | a file that cannot be read or written |
| `internal` | an error in the engine itself |

## Functions

| Function | Options | Result (`ok`) |
|---|---|---|
| `const char *tok_version(void)` | — | the version of the engine, for example `0.7.0` (a static string: not freed) |
| `uint32_t tok_abi_version(void)` | — | the version of the C interface, `TOK_ABI_VERSION` in the header |
| `void tok_free(char *s)` | — | frees a result; `NULL` is ignored |
| `tok_check(text, options)` | `lang` (`en`, `pl`), `level` (`basic`, `analytic`, `full`), `bpmn` (boolean, BPMN rules, default `true`), `class` (`descriptive`, `analytic`, `common-executable`, `full`) | `{"errors": N, "bpmn_class": CLASS, "diagnostics": [{"code", "severity", "line", "id", "message"}, …]}`, as `tok check --json` |
| `tok_fmt(text, options)` | `lang` | the canonical text, as `tok fmt` |
| `tok_bpmn(text, options)` | `lang`, `layout` (boolean, default `true`), `sidecar` (the text of a layout sidecar) | BPMN 2.0 XML, as `tok bpmn` (`--no-layout` with `"layout": false`) |
| `tok_svg(text, options)` | `lang`, `title`, `sidecar` | the SVG picture, as `tok svg` |
| `tok_export(text, options)` | `lang`, `name` (stem of the BPMN and SVG files, default `model`), `sidecar` | an object file name → content: every file `tok export` writes |
| `tok_import(xml, options)` | `format` (`auto`, `bpmn`, `dmn`), `flow_ids` (boolean, default `true`; `false` is `--no-flow-ids`), `layout` (boolean, default `true`) | `{"format": "bpmn" or "dmn", "text": DOCUMENT, "warnings": [...], "layout": SIDECAR or null}` |
| `tok_eval(text, options)` | `decision` (required), `values` (an object input name → JSON value), `lang` | the value of the decision, as `tok eval` prints it |
| `tok_sim(text, options)` | `cases`, `seed`, `lang`, `log` (default `true`), `cases_table` (default `true`), `csv`, `xes` (default `false`) | see [Simulation](#simulation) |
| `tok_sim_model(text, options)` | `cases`, `seed`, `lang` | the simulation model, see [Simulation model](#simulation-model) |
| `tok_sim_dist(text, options)` | none | a distribution of the simulation statements, parsed: `{"name", "kind", "args", "labels"}` |
| `tok_sim_eval(expr, options)` | `vars` (an object of case variables), `lang07` (boolean) | `{"value": VALUE, "truthy": BOOLEAN}`: the expression evaluated as the simulator evaluates conditions and scripts; one that cannot be evaluated is an error of kind `feel` |
| `tok_cli(args, options)` | none | `{"status": EXIT STATUS}`: the `tok` command line with the arguments of a JSON array (`["check", "model.tok"]`), writing to the standard streams of the process |

`tok_check` reports the diagnostics of a document with errors as a successful result (`errors` > 0); the other functions that read a document return an error of kind `document` for it. `tok_fmt` needs only a document without syntax errors; `tok_eval` checks the document without the BPMN rules, as `tok eval` does; the others check it completely, as `tok bpmn` does.

`tok_eval` takes the input values as JSON: numbers, text, booleans, `null`, lists and objects (contexts). In a document of language 0.7 the decision is evaluated with everything it requires and the result is its DMN value (numbers as JSON numbers, dates, times and durations as their FEEL text); in documents of earlier languages the decision table alone gives an object of outputs, a list of them for hit policy `collect`, or `null`.

## Simulation

`tok_sim` runs the simulator of the engine, the same run as `tok sim` for the same document and seed (`cases` and `seed` are numbers or text; they override `simulate`). The result has:

| Member | Content |
|---|---|
| `kpi` | the indicators, as `tok sim --json` prints them under `kpi` |
| `report` | the text `tok sim` prints, in the language of `lang` |
| `log` | (with `log`) one object per activity instance: `case`, `activity`, `label`, `resource`, `enqueue`, `start`, `end` (ISO times to the second, as in `--log`), `status`, and `enqueue_s`, `start_s`, `end_s` (seconds from the start of the simulation, `null` when absent) |
| `cases` | (with `cases_table`) one object per case: `case`, `start`, `end`, `end_event`, `cycle_time_s`, `variables`, as in `tok sim --json`, and `start_s`, `end_s` |
| `log_csv`, `cases_csv` | (with `csv`) the files of `--log` and `--cases-csv`, as text |
| `xes` | (with `xes`) the event log as XES, the file of `--xes` |

A run whose times lie beyond the calendar of the event log (year 9999) cannot give the ISO times of `log` and `cases`; it is reported as an error of kind `simulation`, and the same run with `"log": false, "cases_table": false` gives the indicators.

## Simulation model

`tok_sim_model` gives the simulation model of a checked document as data, for simulators outside the engine (the native simulator of `Tok.jl` is built from it). The object has:

| Member | Content |
|---|---|
| `language`, `lang05`, `lang07` | the language of the header and whether it is 0.5 or later, 0.7 or later |
| `process` | the process the simulator runs (that of the first pool that has one, or the top-level process) |
| `start_event` | its start event (as the simulator chooses it), or `null` |
| `start` | `start=` of `simulate` (or the default `2027-01-04T08:00`) |
| `cases`, `seed` | the number of cases and the seed, as integers |
| `arrival` | the distribution of the time between arrivals |
| `variables` | `[{"name", "dist"}]`, the `variable` statements in order |
| `resources` | `[{"id", "label", "count"}]` |
| `nodes` | every flow node in document order: `id`, `kind`, `label`, `scope` (the process or sub-process it belongs to), `children`, `attrs` (every attribute as text), `flags`, `triggers` (number of triggers of each kind), the parsed distributions `duration`, `delay` and `assign` (`[{"name", "dist"}]`), `timer` and `start` (the start event of a sub-process), each `null` when absent |
| `flows` | the sequence flows between flow nodes, in document order: `id`, `source`, `target`, `when`, `p`, `default` |

A **distribution** is `{"name", "kind", "args", "labels"}`: `name` is `fixed`, `unif`, `tri`, `norm`, `lnorm`, `exp`, `bern` or `cat`; `kind` is `duration` (the arguments are seconds), `number`, `bool` or `cat`; `args` are the arguments in the order of the language (for `tri`: minimum, most likely, maximum; for `cat` the probabilities) and `labels` the labels of `cat` (text, booleans or numbers).

A **timer** is resolved as the simulator resolves it: `{"kind": "duration", "seconds"}` for a duration or a cycle (its first period), `{"kind": "date", "seconds"}` for a date-time (seconds from the start of the simulation; the event waits until then, or not at all when the date has passed), `{"kind": "invalid", "text"}` for a value the simulator does not understand (it warns and does not wait), and `{"kind": "error", "message"}` for a date-time that cannot be compared with the start (one with a UTC offset and one without).

## Example

```c
#include <stdio.h>
#include "tok.h"

int main(void) {
    const char *doc =
        "tok 0.1 \"Example\"\n"
        "start s\n"
        "task a \"Work\" duration=\"exp(1h)\"\n"
        "end e\n"
        "flow s -> a -> e\n"
        "simulate cases=10 seed=1\n";
    char *r = tok_check(doc, "{\"lang\": \"en\"}");
    printf("%s\n", r);    /* {"ok":{"errors":0,"bpmn_class":"descriptive","diagnostics":[...]}} */
    tok_free(r);
    r = tok_sim(doc, "{\"log\": false, \"cases_table\": false}");
    printf("%s\n", r);    /* {"ok":{"kpi":{...},"report":"Simulation (seed 1): 10 cases started..."}} */
    tok_free(r);
    return 0;
}
```

A test program of this kind, `rust/tok-capi/tests/c/example.c`, is compiled and run by the tests of the crate on Unix systems with a C compiler; the integration test `rust/tok-capi/tests/c_api.rs` calls every function through its C signature.

## Versions

`tok_abi_version()` is 1. It changes only when a function is removed or changes its parameters, or when a member of a result changes its meaning; new functions, new options and new members of results keep the version, so a program should ignore members it does not know. A program built against `tok.h` can compare `tok_abi_version()` with `TOK_ABI_VERSION`.

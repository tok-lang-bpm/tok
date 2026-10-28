# Fuzzing

The engine reads text and XML from anywhere: documents written by people and agents, BPMN and DMN files from other tools, layout sidecars, FEEL typed into an editor. Before 1.0 every reader was fuzzed with coverage-guided random inputs, and every failure became a fix and a regression test. This page describes the targets, how to run them, and what the fuzzing found.

## Targets

The targets are in `rust/fuzz`, a cargo workspace of its own (the workspace of the engine excludes it), for [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) and libFuzzer. Each target calls one function of `rust/fuzz/src/oracles.rs`, which states the property checked; a panic, a failed assertion, a stack overflow, a timeout or running out of memory is a finding.

| Target | Input | Property |
|---|---|---|
| `check` | a document | `tok check` at the levels `basic`, `analytic` and `full` with the BPMN rules, and against every conformance class, returns diagnostics |
| `roundtrip` | a document | for a valid document, parse → print → parse → print: the second text equals the first, the printed document has no errors, and it means the same (the semantic BPMN, the decision and form files, the simulation model) |
| `feel` | an expression (at most 512 bytes) | FEEL of languages 0.1–0.6 (the subset, conformance level 2) and of language 0.7 (level 3): parsing and evaluation of expressions, unary tests and types, the names of an expression, the text for Camunda |
| `import_bpmn` | BPMN XML | `tok import` with the layout sidecar of the diagram; the imported text parses without lexer or parser errors and is in canonical form |
| `import_dmn` | DMN XML | the same for DMN |
| `sidecar` | one byte choosing a model, then a layout sidecar | reading the sidecar, BPMN layout and SVG of the model with it |
| `render` | a document (at most 64 KiB) | for a valid document, BPMN with and without layout, SVG and every file of `tok export` |
| `sim` | a document (at most 64 KiB, 60 flow nodes) | for a valid document, three simulated cases with the CSV and XES logs, and the simulation model |

## Running

On Linux (or WSL) with a nightly toolchain and clang:

```text
rustup toolchain install nightly
cargo install cargo-fuzz
python3 rust/fuzz/seeds.py ~/tok-seeds --tok rust/target/release/tok     # seed corpora and dictionaries
cd rust/fuzz
cargo +nightly fuzz build
mkdir -p corpus && cp -r ~/tok-seeds/check corpus/check
cargo +nightly fuzz run check corpus/check -- -dict=$HOME/tok-seeds/check.dict \
    -max_total_time=600 -timeout=10 -rss_limit_mb=2048 -max_len=16384
```

`seeds.py` writes one directory of seed inputs per target and a libFuzzer dictionary: the conformance cases, the examples and the documents of the skill for the document targets; the MIWG models and the BPMN files of the conformance corpus for `import_bpmn`; the DMN TCK models for `import_dmn`; every FEEL expression found in the documents and in the text elements of the DMN models for `feel`; and, with `--tok`, the layout sidecars `tok import` writes for the models of the `sidecar` target. The corpora are not kept in the repository.

A failing input is minimized with `cargo +nightly fuzz tmin TARGET FILE` and goes into `rust/fuzz/regressions/TARGET/`. The test `rust/tok-cli/tests/fuzz_regressions.rs` runs every file there through the same property with the stable toolchain, on Windows, Linux and macOS, as part of `cargo test` and `tools/check.sh`. It runs each input on a thread with the stack of a Windows main thread (1 MiB; 2 MiB for unoptimized builds, whose frames are about twice as large), the smallest stack the engine is called on, so a recursion that only fits a larger stack fails there.

## What was found and fixed

The first runs, with the seed corpora, found the same thing in every reader within minutes: **recursion without a bound**. The parser of the language, the checker, the printer, the exporters and the layout recurse into the blocks of a document; the two FEEL parsers recurse into parentheses, lists, contexts, calls, conditionals and unary minus, and the evaluators into the tree of an expression; the importers and the XML tree of the BPMN tools recurse into elements. A few thousand nested parentheses, sub-processes or XML elements overflowed the stack and killed the process, which for the Python, R and Julia packages and the C interface means the host program. Even a flat expression could do it: `1 + 1 + … + 1` is parsed by a loop, but the tree it builds is as deep as the chain is long, and the evaluator walks it recursively. The nesting is now bounded where it enters the engine, with a diagnostic instead of a crash:

- blocks nested more than 64 levels deep are `P008` ("its block is nested more than 64 levels deep");
- FEEL expressions nested more than 160 levels deep are `F002` ("the expression is nested too deeply"); every nested part and every operator of a chain counts one level, so the bound limits the height of the tree, which bounds the parser, the evaluator and everything else that walks an expression;
- BPMN and DMN files whose elements are nested more than 64 levels deep are refused by the import (the deepest of the MIWG models and the DMN TCK nests 12 levels); the BPMN tools accept 256 levels, more than the files tok writes for the deepest documents it reads.

The bounds were chosen with the regression tests: a document, an expression and a file just within each bound run in an unoptimized build on 2 MiB of stack and in an optimized build on 1 MiB, and just beyond it give the diagnostic. Nothing in the conformance corpus, the examples, the MIWG models or the DMN TCK comes near them.

With the overflows gone, the fuzzers found the bugs behind them:

- **An endless recursion in the FEEL parser of language 0.7.** `for`, `some` and `every` start an iteration or a quantifier only when a name follows; otherwise the primary expression handed the keyword back to the expression parser, which handed it back to the primary expression. `for[`, `some""` and `every/list<` (six to eleven bytes) recursed until the stack ran out. Such a keyword is a syntax error now.
- **Exponential parsing in the FEEL subset of languages 0.1–0.6.** The target of `in` is parsed once to see whether it is an interval and once more for real; a nested `a in [b in [c in […]]]` was parsed 4ⁿ times. The parser remembers the targets it has read.
- **Quadratic iteration in FEEL.** `for` gives its body the results so far as `partial`, and copied them in every iteration. A range of a few hundred thousand days (`for i in @"1980-01-03"..@"1280-01-01" return i`) ran for minutes. The copy is made only for a body that reads `partial`.
- **BPMN files without identifiers, or with repeated ones.** The import gave such elements the empty identifier, or the identifier of the element before them, or the text `None`; the printed document did not parse (`start  "Start"`, `boundary  on=None`, `flow  -> e`). Worse, a transaction containing an element with its own identifier became a node containing itself, and the export recursed without end. Every element the import declares now gets an identifier of its own (named after its element, `startEvent`, `task-2`), references to a repeated identifier lead to the first element that has it, and both are reported. A message flow without both ends is dropped and reported.
- **DMN cells that a row cannot hold.** A decision table whose output was named `a=b` (or whose input expression contained `=`, or a cell `|` or `#` outside a string) was printed as a header the parser reads differently. Such a table is not kept and the import reports it, as for other DMN content tok cannot express.
- **A time with letters outside ASCII.** `time("121[2N.Қ]:233:]…")` panicked: the reader of times takes the hours, minutes and seconds by byte position, and a position fell inside a character of two bytes. A time whose text before a zone identifier is not ASCII is not a time (`null`), as every such text was already.
- **Decision tables with an empty side.** A table may have no input columns (`|| out: number |`) or no output columns; the printer wrote the empty side as an empty cell (`|  || out: number |`), which reads back as a column without a name (`D009`). The parser now records that a side of the header has no cells, and the printer leaves that side out; every other table prints as before, including a header whose cells were all malformed, whose canonical form the conformance corpus fixes (cases 15, 16, 59, 60).
- **Very large coordinates in a layout sidecar.** The grids added for performance (`performance.md`) count the cells a shape covers; a shape a billion units wide would have covered billions of cells, and the product of two such counts could overflow. Such shapes are kept in a list every query returns, and the counts saturate. Found while writing the regression inputs of the `sidecar` target.

The property of the `roundtrip` target needed four corrections of its own, all found on valid documents: the canonical form prints the sequence flows of a choreography in its block, so the order of the flows changes between choreographies and processes but not among the flows that leave one node, which is all the simulator depends on; attributes are printed in canonical order; the entries of `listeners=` are printed with the execution listeners first and without surrounding white space, which the export ignores; and the DMN files of a 0.7 document come in the order of the first elements of their graphs, which changes when the canonical form puts the input data first. The oracle compares the simulation model without these differences and the DMN files by name. The dictionary of the `sidecar` target was rejected by libFuzzer at first: dictionaries know the escapes `\\`, `\"` and `\xHH` only, so `seeds.py` writes a line break as `\x0A`.

No finding needed a change of any output of the conformance corpus, the examples, the MIWG models or the DMN TCK: every gate passes unchanged.

## Time spent

The fuzzing ran on one workstation (AMD Ryzen 9 6900HS, Linux under WSL 2) with AddressSanitizer and the debug assertions of cargo-fuzz. Two first short campaigns ran all eight targets at once, two workers each, from the seed corpora; they found the stack overflows, the recursion of `for` and the identifier problems of the BPMN import. After the fixes every target ran alone, with one worker on at most four cores, for runs of ten minutes (a run that stops at a finding is shorter; the next starts from the corpus it left):

| Target | Time | Runs | Executions |
|---|---|---|---|
| `check` | 20 min | 2 × 10 min | 171 000 |
| `import_bpmn` | 20 min | 2 × 10 min | 457 000 |
| `import_dmn` | 20 min | 2 × 10 min | 1 030 000 |
| `roundtrip` | 18 min | 3 (stopped at the empty side of a table and at two corrections of the oracle) | – |
| `feel` | 11 min | 3 (stopped at the long iteration, the time with letters outside ASCII and the large power, below) | – |
| `sidecar` | 10 min | 1 | 227 000 |
| `render` | 10 min | 1 | 275 000 |
| `sim` | 10 min | 1 | 523 000 |

About two hours in all, besides the first campaigns. The corpora (not minimized) grew from 104 seed documents to between 4 500 and 10 300 inputs for the document targets, from 93 BPMN files to 4 900, from 160 DMN files to 5 300, from 4 200 FEEL expressions to 9 500 and from 26 sidecars to 1 200. The last runs of every target ended without a finding, except `feel`, whose last run stopped at `10 + 2**077777**07777777777777`: raising 2⁷⁷⁷⁷⁷ to a large power takes the logarithm of a number with 23 000 digits, which the engine computes by halving it 77 777 times (0.8 s natively; more than the ten seconds of the timeout under the sanitizer). The result (`null`, the power overflows) is right; the time is the limit described below.

## Limits

- Evaluation is not bounded in time: a FEEL expression may describe a large computation (`for i in 1..100000000 return i`), and the evaluator does it. The fuzzing timeout flagged such inputs; they run in linear time. The logarithm (and with it a power with an exponent that is not a whole number below 10⁹) of a number with a decimal exponent in the tens of thousands takes up to a second, because the argument is halved into [0.75, 1.5) one step at a time; a faster reduction would change the last digits of results, so it waits for a release that may change them.
- The `render` and `sim` targets need valid documents, which random mutation rarely keeps; they mostly explore the documents of the seed corpus and their small variations.
- Miri was not run: the crates with fuzz targets contain no `unsafe` code (the workspace forbids it), and the C interface, which has some, is covered by its own tests.

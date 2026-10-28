# Performance

The engine was measured on large generated models before 1.0, and the operations whose time grew faster than the size of the model where it matters were rewritten. Every rewrite gives the same output: the drawings, diagnostics and files of the conformance corpus, the examples, the MIWG models and every generated model compared below are byte for byte those of 0.9.0.

## Benchmarks

The crate `rust/tok-bench` generates large valid models and times the engine on them:

```text
cargo bench -p tok-bench                         every benchmark (a few minutes)
cargo bench -p tok-bench -- process/layout       the benchmarks whose name contains the text
cargo bench -p tok-bench -- process/layout/1000  one size exactly
cargo bench -p tok-bench -- --quick              one run of each
cargo run --release -p tok-bench --example generate -- process 10000 > big.tok
```

| Model | Generator | Contents |
|---|---|---|
| `process/N` | `tok_bench::process(N)` | a process of about N flow nodes: blocks of a user task, an exclusive split with two branches and a join; a timer boundary event with its own path on every seventh task; a data object read and written every tenth block; simulation parameters on every activity |
| `collaboration/N` | `tok_bench::collaboration(pools, lanes, tasks)` | pools of lanes of tasks (N tasks in all), one chain through the lanes of each pool, and a message from each pool to the next |
| `nested/N` | `tok_bench::nested(N)` | sub-processes nested N levels deep |
| `table/N` | `tok_bench::decision_table(N)` | a decision table of language 0.7 with N rules (`hit=first`), called by a rule task |
| `graph/N` | `tok_bench::graph(N)` | a decision requirements graph of N decisions, each requiring the one before it and calling a business knowledge model |
| `feel/N` | `tok_bench::feel(N)` | a FEEL expression of N conditional terms in a list, filtered, iterated and summed |

Each benchmark runs until it has taken about a second (at least three times, at most a hundred) and gives the median. The operations: `parse` (the parser), `check` (`tok check` with the BPMN rules), `fmt` (the canonical printer), `export` (semantic BPMN), `layout` (BPMN with the automatic diagram), `svg`, `import` (the laid-out BPMN back to tok), `import-sidecar` (the import with the layout sidecar, as `tok import -o`), `eval` (`tok eval` of a decision), `dmn` (the DMN files), `sim` (100 simulated cases).

The test `rust/tok-bench/tests/generated.rs` checks that the generated models are valid and that their canonical form is a fixed point; it runs with the other tests. There is no timing test in the suite: the times of a shared machine vary too much for a bound that would catch a regression without failing at random.

## Results

Measured on an AMD Ryzen 9 6900HS (8 cores, 16 threads, one used), 32 GB, Windows 11, Rust 1.99.1, release build (`cargo bench`), with the generators above, while a fuzzing run used four other cores; the median time of one run. The size is the number of flow nodes (`process`, `nested`: the depth), tasks, rules, decisions or terms.

**process**

| Size | parse | check | fmt | export | layout | svg | import | import-sidecar |
|---|---|---|---|---|---|---|---|---|
| 100 | 0.60 ms | 2.2 ms | 0.17 ms | 0.50 ms | 6.3 ms | 2.2 ms | 0.72 ms | 4.1 ms |
| 1000 | 4.9 ms | 25 ms | 1.7 ms | 7.8 ms | 90 ms | 33 ms | 11 ms | 62 ms |
| 10000 | 69 ms | 698 ms | 24 ms | 322 ms | 1.30 s | 344 ms | 136 ms | 982 ms |

**collaboration**

| Size | parse | check | fmt | export | layout | svg | import | import-sidecar |
|---|---|---|---|---|---|---|---|---|
| 100 | 0.57 ms | 2.3 ms | 0.13 ms | 0.52 ms | 7.4 ms | 2.1 ms | 0.86 ms | 4.3 ms |
| 1000 | 3.4 ms | 18 ms | 1.0 ms | 4.2 ms | 65 ms | 20 ms | 8.0 ms | 41 ms |

**nested**

| Size | parse | check | fmt | export | layout | svg | import | import-sidecar |
|---|---|---|---|---|---|---|---|---|
| 10 | 0.12 ms | 0.71 ms | 0.04 ms | 0.15 ms | 2.3 ms | 0.65 ms | 0.28 ms | 1.4 ms |
| 60 | 0.71 ms | 3.7 ms | 0.26 ms | 0.91 ms | 55 ms | 3.7 ms | 1.6 ms | 8.0 ms |

**simulation, decisions and FEEL**

| Benchmark | Sizes | Times |
|---|---|---|
| `sim/process` | 100, 1000 | 25 ms, 311 ms |
| `table/check` | 100, 1000, 10000 | 1.0 ms, 7.8 ms, 79 ms |
| `table/eval` | 100, 1000, 10000 | 0.33 ms, 2.7 ms, 31 ms |
| `table/dmn` | 100, 1000, 10000 | 0.85 ms, 8.7 ms, 130 ms |
| `graph/check` | 10, 100, 500 | 0.44 ms, 2.3 ms, 23 ms |
| `graph/eval` | 10, 100, 500 | 0.17 ms, 0.58 ms, 0.92 ms |
| `graph/dmn` | 10, 100, 500 | 0.52 ms, 8.4 ms, 134 ms |
| `feel/parse` | 100, 1000, 10000 | 0.43 ms, 4.5 ms, 53 ms |
| `feel/eval` | 100, 1000, 10000 | 0.20 ms, 2.0 ms, 24 ms |
| `feel/check` | 100, 1000, 10000 | 1.1 ms, 9.1 ms, 111 ms |

Most operations grow linearly with the model. Parsing, formatting, import, decision evaluation and FEEL handle 10 000 elements, rules or terms in a fraction of a second. What still grows somewhat faster than the model, all within about a second at 10 000 elements: the semantic BPMN export (`export`, 8 ms at 1 000, 0.32 s at 10 000: the exporter finds elements of the XML tree by identifier with a search from the root), `check` (which exports the document and runs the BPMN rules over it), the import with the layout sidecar (which exports and compares the drawing), the layout (89 ms at 1 000, 1.3 s at 10 000) and the DMN files of a long graph (`graph/dmn`). The simulation of 100 cases grows with the length of the process, as every case walks it.

## What was changed

The first measurements showed one operation that did not scale: the layout. A process of 10 000 elements took 93 s to lay out (100 elements: 11 ms; 1 000: 0.66 s), almost all of it in work done for every flow or label over the whole diagram. The checker of decision requirements graphs was the other: a graph of 500 decisions took 0.94 s to check (100: 12 ms), its time growing with the cube of the size. Both were rewritten without changing a result:

| Benchmark | 0.9.0 | Now |
|---|---|---|
| `process/layout/100` | 11 ms | 6.3 ms |
| `process/layout/1000` | 0.66 s | 90 ms |
| `process/layout/10000` | 93 s | 1.3 s |
| `graph/check/100` | 12 ms | 2.3 ms |
| `graph/check/500` | 0.94 s | 23 ms |

- **Routes of the classic layout.** Every sequence flow is routed by scoring a few candidate routes against the shapes they would cross. The obstacles were collected for every flow by a scan of every shape of the diagram (with a lookup of its element), and every candidate was scored against all of them. The shapes are now collected once per layout, with a grid of their boxes; a candidate is scored against the shapes in the cells it passes, and the column of shapes above and below a flow comes from the grid as well. A route counts the shapes it crosses, and a shape outside the cells of a route cannot be crossed by it, so the scores are the same.
- **Barycentre sweeps.** The nodes of each column and lane were picked out of all nodes for every column, lane and sweep; they are now grouped once.
- **Refinement.** The refinement (`refine.rs`) measures a route or a label against the shapes, routes and labels near it: crossings, overlaps, labels on routes, labels on labels. These loops ran over the whole diagram for every candidate of every flow and every label. The scene keeps three grids — of the boxes of the shapes, of the routes and of the labels — updated when a route or a label moves, and every measure looks only at the items in the cells it covers. Every measure counts what it finds, and nothing outside those cells can be found, so the counts, and with them every decision of the refinement, are the same. The same grids bound the router on the grid of free channels to the shapes, routes and labels near its window.
- **Cycles of requirements (`R076`).** For every element of a graph, a depth-first search looked for a path back to it, with a list of the elements seen (a linear search) and a copy of the path for every element reached. Now the elements that lie on a cycle are found first, in one pass over the strongly connected components, and only for them does the search run, with a set of the elements seen and the path kept as links back to where each element was reached. The reported paths are those of the old search.
- **FEEL iteration.** `for` copied the results so far (`partial`) in every iteration; the copy is made only when the body reads `partial` (found by fuzzing, `fuzzing.md`).

The outputs were compared before and after on the conformance cases, the examples, the Camunda cases, the MIWG models and the BPMN files of the course imported with their sidecars, and on generated processes of up to 1 000 elements, collaborations and nested sub-processes: 176 BPMN and SVG files, byte for byte equal. The golden layouts, the layout quality measures and the sidecar tests of `tok-bpmn` pass unchanged.


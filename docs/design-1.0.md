# Design: language 1.0 (the stable language)

Status: **accepted** (2026-10-28) and implemented; the language is specified in `spec.md` (§2.1, §4.1–§4.3, §15) and its diagnostics in `diagnostics.md`. Language 1.0 (`tok 1.0` in the header) is language 0.7 made stable: the same statements, attributes and flags, the limits that the hardening of the engine introduced written as rules, one deprecation, and the promise of what every 1.x release keeps. Documents for 0.1, 0.3, 0.5, 0.6 and 0.7 remain valid, mean the same and keep every output byte for byte, except where an output was wrong (decision 6).

## Principles kept

1. Nothing is removed. Every construct of languages 0.1–0.7 is part of 1.0 with its meaning.
2. Nothing is added to the syntax. A 1.0 document is a 0.7 document with another header.
3. The outputs of existing documents do not change: the conformance cases 01–61, the examples, the MIWG models and the DMN TCK give the same files as before, and none of their expected files is regenerated.
4. What a later version may refuse is announced in 1.0 as a warning, and only to documents that declare 1.0.

## 1. The header

`tok 1.0` is accepted next to `0.1`, `0.3`, `0.5`, `0.6` and `0.7`. The version is a number with its minor part: `tok 1` is an unknown version (`P006`, a warning), as `tok 1.1` is; a version that is not a number (`tok v1.0`) is `P005`, and a second header is `P010`, as before. A 1.0 document has everything that depends on the declared version: several triggers on an event and BPMN resources and performers (0.5), participant multiplicity (0.6), and the statements of decision requirements graphs, boxed expressions and FEEL at conformance level 3 with numbers of 34 digits (0.7).

## 2. Outputs of a 1.0 document

| Output | 1.0 document |
|---|---|
| BPMN | as the same document with the header `tok 0.7`; `exporterVersion="1.0"` |
| DMN | DMN 1.5 (`https://www.omg.org/spec/DMN/20230324/MODEL/`, DMNDI `https://www.omg.org/spec/DMN/20230324/DMNDI/`), one file per decision requirements graph as in 0.7; `exporterVersion="1.0"` |
| Camunda 8 | in an executable document for Camunda, `modeler:executionPlatformVersion="8.10.0"` in BPMN, DMN and forms and FEEL names of several words in backticks, as in 0.7 |
| forms | as in 0.7: the form files carry no language version (`exporter.version` stays `0.1.0`, the version of the form exporter since 0.1) |
| SVG, layout sidecar | as in 0.7 |
| `tok eval`, `tok sim` | as in 0.7 |
| canonical form | as in 0.7, with the header `tok 1.0` |

## 3. Limits as rules

The hardening before 1.0 bounded the nesting the engine reads, because every reader recursed without a bound (`fuzzing.md`). The bounds are now part of the specification (§2.1) and hold for documents of every version, since they were bug fixes of crashes, not changes of the language: blocks at most 64 levels deep (`P008` beyond), FEEL expressions at most 160 levels deep (`F002` beyond), BPMN and DMN files read by the import at most 64 levels deep, and XML files read by the BPMN tools at most 256 levels deep. A conforming implementation reads everything within the bounds and reports what lies beyond them without failing.

## 4. Review of the language

Every construct was reviewed for names that do not fit and for redundant forms. Nothing is removed (principle 1); what is kept and why:

| Construct | Observation | Decision |
|---|---|---|
| a key or flag written twice on one line (`doc="a" doc="b"`, `loop loop`) | redundant: the earlier values have always been dropped silently, and `tok fmt` removes them | **deprecated** in 1.0: `R090` (warning) in 1.0 documents |
| the trigger keys repeated on an event, `error=` repeated on an operation | the values all count (multiple events, language 0.5) | kept, not reported; on another flow node the repetition stays `R062` |
| `noninterrupting`, the only flag of several words without a hyphen (`test-before`, `loop-max`, `follow-up`) | an inconsistent name | kept: an alias would be a second form of the same thing, which the canonical form avoids |
| `type=` on service tasks (a job type), on data (an item), on DMN elements (a FEEL or DMN type) and on form fields (a component) | one key with four meanings | kept: each meaning belongs to one kind of statement, and the checker reports a wrong value where it stands |
| `method=` on a transaction (`compensate`, `store`, `image`) and on the REST connector (`GET`, …) | one key with two meanings | kept, as above |
| `version=` (the binding of a call, a decision or a form) and `version-tag=` (the version tag of the process) | similar names for different things | kept: they follow the names of Camunda Modeler |
| `parallel` on activities (multi-instance), on events (parallel multiple) and on `eventgw` (parallel event-based gateway) | one flag with three meanings | kept: each follows the BPMN attribute of its element |
| `variable NAME "DIST"` and `text "…"`, whose quoted value stands where a label stands elsewhere | a positional value | kept: these statements have no label |
| the decision tables of languages 0.1–0.6 (DMN 1.3, the object of outputs from `tok eval`) | another evaluation than 0.7 | kept for documents of those versions; a 1.0 document has the decision tables of 0.7 |

The messages of `P005` ("the document must start with the header 'tok 0.1'") and `P006` ("…; this tool reads version 0.1") still name version 0.1. They are kept as they are, because changing them would change the output of `tok check` for existing documents and they are shared with the frozen pure-Python implementation; a later version may correct them as a documented fix.

## 5. Compatibility of 1.x

The specification promises (§4.3): every valid 1.0 document stays valid and prints identically in every 1.x release; outputs change only for bug fixes documented in the changelog, with the expected files of the conformance corpus; new revisions of Camunda 8 and of DMN and FEEL come in minor releases, and what they change in the meaning or outputs of a document comes with a new language version in the header (`tok 1.1`), so that 1.0 documents keep their outputs; documents of 0.1–0.7 keep theirs. A break of any of this is reserved for 2.0.

## 6. Conformance

Cases 62–67 cover language 1.0: a document with the elements of every layer — pools and lanes, every kind of event and gateway, a transaction with compensation, a standard loop, a multiple start event, a global task, an operation, process data, a correlation key, conversations, a choreography, participant multiplicity, artifacts, a decision requirements graph with a DMN type, a boxed context with a relation, a form and simulation parameters (62); an executable document for Camunda 8 with forms, a script in FEEL of level 3, a job, a user task and a decision model with a name of two words (63); the errors of the header: a version that does not exist with a second header (64), a version without its minor part (65), a version that is not a number (66); and the deprecation `R090` with the keys that may repeat (67). Their expected outputs are produced by the Rust engine; the Julia core passes the core level for them; the pure-Python implementation, frozen at 0.3, reports a 1.0 header as an unknown version (`P006`) as it reports 0.5–0.7, and its parity tests cover the cases of 0.1 and 0.3 as before.

## Decisions taken during implementation

1. **1.0 is 0.7 with another header.** Every predicate of the engine that asks whether a document is of 0.5, 0.6 or 0.7 or newer is true for 1.0; nothing else in the readers, checker, printer, exporters, evaluator or simulator looks at the version. Only the deprecation of §4 asks for 1.0 exactly, so an unknown version (`tok 1.1`, read with every statement of 0.7 after its `P006`) gets no deprecation warnings and keeps the diagnostics it had.
2. **`exporterVersion` is the language version of the document,** as for every earlier version: `1.0` in the BPMN and DMN files of a 1.0 document. The DMN version stays DMN 1.5, the newest revision Camunda 8.10 reads.
3. **The import keeps its versions.** `tok import` writes `tok 0.1`, `0.3`, `0.5` or `0.6` for BPMN files and `tok 0.7` for DMN files, the oldest version that expresses what it read, as before; each of these documents is a valid 1.x document with the same meaning, and writing `tok 1.0` would change the outputs of every import.
4. **One deprecation code, `R090`.** It is a warning of the checker, reported at every level (the `.levels` file of case 67 shows it), not of the parser: it is not a core code, so the independent cores need not report it. The parser records the repeated keys and flags of every document; the checker reports them only for 1.0 documents, and `tok fmt` removes them by printing the last value of a key and one copy of a flag, as it always did.
5. **The cases of the Python generator.** `TOK_BLESS=1 cargo test -p tok-core --test conformance` now writes the expected files of every case that `conformance/generate.py` skips, by the same rule (the second word of the first statement line is not `0.1` or `0.3`), instead of the cases the engine reads as 0.5 or newer. For the cases 01–61 the two rules select the same cases; the new rule also covers a header that is not a number (case 66), which neither generator wrote before.
6. **A bug found by case 62.** In a document with pools, groups and conversations the BPMN export put the conversations into the `category` of the groups instead of the collaboration: the category was inserted at the start of the definitions while the positions of the collaboration and the processes were still in use, so every later step that wrote into them by position wrote one element too early. The category is now placed with the declarations at the end of the export. No case of the corpus and no example had both groups and these later elements, so no expected file changes; the Julia core, which places the elements by reference, already wrote them correctly.
7. **A bug of the language server found by case 62.** Renaming a business knowledge model `kara` also renamed the first word of the context entry `kara za tydzień` and the key `kara:` of a context, and renaming a DMN type did not rename it in the header of a decision table. The server now skips the words of a longer FEEL name of the document (FEEL reads the longest name) and the keys of contexts, and follows the column types of table headers.
8. **Camunda.** `conformance/camunda/run.py` runs the process of case 63 (exported with the native `tok`) on Camunda 8 Run 8.10.7 after the process cases of language 0.3, and evaluates its decisions with those of case 61 at the end, since the decision evaluations come last by design.

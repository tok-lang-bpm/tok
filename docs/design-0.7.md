# Design: language 0.7 (decisions and expressions)

Status: **accepted** (2026-10-28) and implemented; the language is specified in `spec.md` (§4, §9, §10.4, §11, §12, §13–§15) and its FEEL in `feel.md` §9; the decisions taken during implementation are listed at the end. Language 0.7 (`tok 0.7` in the header) turns the decision tables of earlier versions into complete DMN 1.5 decision models: decision requirements diagrams, boxed expressions and FEEL at conformance level 3. Documents for 0.1, 0.3, 0.5 and 0.6 remain valid and mean the same; their outputs do not change.

## Principles kept

1. Every statement starts with a keyword; one statement per line; blocks by indentation.
2. New keywords only for new kinds of things: the elements of a decision requirements graph that tok did not have (`inputdata`, `bkm`, `source`) and the parts of boxed expressions that are lines of a block (`entry`, `result`, `element`). Everything else is an attribute or a flag of an existing statement (`requires=`, `expr=`, `invoke=`, `with=`, `params=`, `type=`, `name=`).
3. The simple syntax of descriptive models does not change, and neither does the decision table of language 0.1: a table written for 0.1 means the same in 0.7.
4. The executable layer stays optional. A decision requirements diagram without decision logic is a valid descriptive model; `tok eval` gives `null` for a decision without logic.
5. Decisions, as before, belong to the level `full`; the levels `basic` and `analytic` reject the new statements (`R040`).
6. The DMN version is **DMN 1.5** (namespace `https://www.omg.org/spec/DMN/20230324/MODEL/`) for 0.7 documents. Documents of earlier versions keep the DMN 1.3 export they had.

## 1. Decision requirements diagrams

```
tok 0.7 "Wypożyczenia w bibliotece"

inputdata wiek "Wiek czytelnika" type=number
inputdata konto "Rodzaj konta" type=string
inputdata zaleglosci "Zaległe opłaty (zł)" type=number

source regulamin "Regulamin wypożyczalni" kind="regulamin" uri="https://biblioteka.example/regulamin"

bkm kara "Kara za zwłokę" params="dni: number, stawka: number" type=number expr="dni * stawka"

decision limit "Limit wypożyczeń" requires="wiek konto regulamin" hit=first
  | wiek: number | konto: string          || limit: number |
  | < 18         | -                      || 5             |
  | >= 18        | "student", "doktorant" || 15            |
  | -            | -                      || 10            |

decision blokada "Czy zablokować konto?" requires="zaleglosci limit" type=boolean expr="zaleglosci > 50 or limit < 1"
```

| tok | DMN 1.5 |
|---|---|
| `inputdata ID ["Label"] [name="N"] [type=T]` | `inputData` with its `variable` |
| `decision ID ["Label"] [name="N"] [type=T] [requires="ID …"] [question="…"] [answers="…"]` + decision logic | `decision` with `variable`, `question`, `allowedAnswers`, requirements and its expression |
| `bkm ID ["Label"] [name="N"] params="p: T, …" [type=T] [requires="ID …"]` + logic | `businessKnowledgeModel` with `variable` and `encapsulatedLogic` (a `functionDefinition` with `formalParameter`s) |
| `source ID ["Label"] [name="N"] [requires="ID …"] [kind="…"] [uri="…"]` | `knowledgeSource` with `type` and `locationURI` |
| `requires=` naming a decision or input data | `informationRequirement` (`requiredDecision`, `requiredInput`) |
| `requires=` naming a business knowledge model | `knowledgeRequirement` (`requiredKnowledge`) |
| `requires=` naming a knowledge source | `authorityRequirement` (`requiredAuthority`; on a knowledge source also `requiredDecision`, `requiredInput`) |
| label | `label` of the element (when it differs from the name) |
| `doc="…"` | `description` |

- **Names.** The name of an element is its variable name in FEEL: the identifier, or `name="…"` when the name is not a tok identifier (DMN names may contain spaces: `name="Ocena ryzyka"`). Other decisions refer to it by that name: `limit < 1`.
- **Requirements.** `requires=` lists the elements an element depends on; the kind of each named element gives the kind of requirement. A decision requires decisions, input data, business knowledge models and knowledge sources; a business knowledge model requires business knowledge models and knowledge sources; a knowledge source requires decisions, input data and knowledge sources; input data requires nothing (`R003` otherwise). The graph is acyclic (`R076`). A decision sees only what it requires: the values of required decisions and input data and the functions of required business knowledge models.
- **Diagram.** Each connected decision requirements graph is exported as one DMN file with a DMNDI diagram laid out automatically: decisions above what they require, requirement edges pointing up (solid with a filled arrow for information, dashed for knowledge, dashed with a dot for authority), shapes in the DMN notation (rectangles, rounded input data, cut-corner business knowledge models, knowledge sources with a wavy base).

## 2. Boxed expressions

The logic of a decision, of a business knowledge model, of a context entry and of a list element is one boxed expression. It is given by an attribute or by the block of the statement:

| tok | DMN 1.5 |
|---|---|
| `expr="FEEL"` | `literalExpression` |
| block of table rows with `\|\|` (and `hit=`, `aggregation=` on the statement) | `decisionTable` |
| block of table rows without `\|\|` | `relation`: the header row names the columns, each further row is a row of literal expressions |
| block of `entry NAME [type=T] …` lines, optionally ending with `result [type=T] …` | `context` with `contextEntry`s; `result` is the final entry without a variable |
| block of `element …` lines | `list` |
| `invoke=BKM [with="p = FEEL; …"]` | `invocation` of the business knowledge model with one `binding` per parameter |
| `params="p: T, …"` on `entry`, `result` or `element` | `functionDefinition` whose body is the logic of the line |

```
decision oplata "Opłata za przetrzymanie" requires="dni-zwloki rodzaj kara stawki" type=number
  entry stawka type=number expr="stawki[typ = rodzaj].stawka[1]"
  entry naliczona invoke=kara with="dni = dni-zwloki; stawka = stawka"
  entry zaokraglenie params="x: number" expr="round half up(x, 2)"
  result expr="zaokraglenie(min(naliczona, 100))"

decision stawki "Stawki opłat"
  | typ: string  | stawka: number |
  | "książka"    | 0.5            |
  | "czasopismo" | 0.2            |

decision kategorie "Kategorie kont"
  element expr="\"student\""
  element expr="\"doktorant\""
  element expr="\"pracownik\""
```

- An entry or element whose value is a table, a relation, a context or a list carries the rows or lines in its own block: `entry stawki` followed by indented rows.
- The name of an entry is a FEEL name: a word, or a quoted string when it contains spaces (`entry "do zapłaty" expr="…"`). Entry names are local to their context and are not document identifiers.
- Exactly one form of logic per statement (`R077`); `with=` requires `invoke=`, and its parameters are parameters of the business knowledge model (`R078`).

## 3. Decision tables in language 0.7

The table syntax of 0.1 is unchanged. Language 0.7 adds:

| tok | DMN 1.5 |
|---|---|
| `hit=rule-order`, `hit=output-order` | hit policies `RULE ORDER`, `OUTPUT ORDER` |
| `hit=collect aggregation=sum\|count\|min\|max` | `COLLECT` with `aggregation` |
| header cell `name: type = "a", "b", …` | `inputValues` of an input clause, `outputValues` of an output clause (unary tests); `priority` and `output-order` order the outputs by them (`D006`) |
| `default="FEEL"` (one output), `default="output = FEEL; …"` | `defaultOutputEntry` of the output clauses |
| header types: all FEEL types (`string`, `number`, `boolean`, `date`, `time`, `date and time`, `days and time duration`, `years and months duration`, `list`, `context`, `any`) and item names (§6) | `typeRef` |

`priority` evaluates as DMN defines it (in earlier documents it behaves like `first`, as before). In a 0.7 document the value of a decision table with one output is the value of that output, with several outputs a context of the outputs, with `collect`, `rule-order` and `output-order` a list of them, with an aggregation the aggregated value.

## 4. FEEL at conformance level 3

Every FEEL text of a 0.7 document (decision logic, conditions, scripts, all attributes of the executable layer) is full FEEL of DMN 1.5:

- **Values**: numbers, strings, booleans, `null`, dates, times, dates and times (with offsets, `Z` or zone identifiers), days-and-time and years-and-months durations, lists, contexts, ranges and functions; `@"…"` temporal literals; numbers in scientific notation (`1.23e4`).
- **Arithmetic and comparison** of numbers and temporal values as DMN 1.5 §10.3.2 defines (`date - date`, `date and time + duration`, `duration * number`, …); `**`.
- **Decimal semantics**: numbers are decimal with **34 significant digits** and rounding half even (IEEE 754 decimal128, as DMN requires). Earlier documents keep the 28 digits of tok 0.1–0.6, so their results do not change.
- **Expressions**: contexts (`{a: 1, "b c": 2}`), path expressions (`a.b`, over lists too), filters (`l[item > 1]`, `l[1]`, `l[-1]`), `for … in … return`, `some`/`every … satisfies`, `instance of` with type expressions, `function(x) …` definitions, named and positional arguments, ranges and unary tests everywhere, comments `//` and `/* */`.
- **Names with spaces** and with the additional symbols `. / - ' + *` are resolved against the names in scope (DMN 1.5 §10.3.1.2): `Ocena ryzyka + 1`, `dni-zwloki * 2`.
- **The standard function library** of DMN 1.5 §10.3.4: conversion (`date`, `time`, `date and time`, `duration`, `years and months duration`, `number`, `string`), boolean (`not`), string (`substring`, `string length`, `upper case`, `lower case`, `substring before`, `substring after`, `replace`, `contains`, `starts with`, `ends with`, `matches`, `split`, `string join`), list (`list contains`, `count`, `min`, `max`, `sum`, `mean`, `all`, `any`, `sublist`, `append`, `concatenate`, `insert before`, `remove`, `reverse`, `index of`, `union`, `distinct values`, `flatten`, `product`, `median`, `stddev`, `mode`, `sort`, `list replace`), numeric (`decimal`, `floor`, `ceiling`, `round up`, `round down`, `round half up`, `round half down`, `abs`, `modulo`, `sqrt`, `log`, `exp`, `odd`, `even`), temporal (`day of year`, `day of week`, `month of year`, `week of year`, `last day of month`, `now`, `today`, `is`), range (`before`, `after`, `meets`, `met by`, `overlaps`, `overlaps before`, `overlaps after`, `finishes`, `finished by`, `includes`, `during`, `starts`, `started by`, `coincides`, `range`), context (`get value`, `get entries`, `context`, `context put`, `context merge`).
- Errors are values: an operation that is not defined for its operands gives `null`, as DMN specifies.

`F002` reports FEEL syntax errors; `F001` reports FEEL that tok cannot evaluate (externally defined functions).

## 5. Evaluation

`tok eval FILE DECISION name=value …` evaluates a decision of a 0.7 document with everything it requires: required decisions (each evaluated once), input data from the command line by name, the functions of business knowledge models. A value is `true`/`false`, a number, a JSON object or list, or else text; text given for input data of a temporal type is read as that type (`termin=2027-01-15` for `type=date`). The result is the value of the decision as JSON (dates, times and durations as their FEEL strings). A violated hit policy in the decision or in a decision it requires is reported with exit code 1, as before.

`tok sim` uses the same evaluator for business rule tasks of 0.7 documents: the case variables are the input data, and the result is stored in `result=` (default: the only output of a decision table, otherwise the name of the decision). The Python function `tok.api.eval_decision` and the R function `tok_eval` evaluate decision requirements graphs the same way.

## 6. Types

```
item czytelnik-t "Czytelnik" type=context
  item wiek type=number allowed="[0..150]"
  item konto type=string allowed="\"student\", \"doktorant\", \"pracownik\""
item kwoty type=number collection
inputdata czytelnik "Czytelnik" type=czytelnik-t
```

In a 0.7 document an `item` with `type=` or with a block of component lines is a DMN type (`itemDefinition` with `typeRef`, `allowedValues`, `isCollection` and `itemComponent`s). It is exported to the DMN files, not to BPMN. `type=` of DRG elements, entries, parameters and table columns names a FEEL type or such an item. Values that do not conform to the type of an input data, a decision or a business knowledge model become `null` (DMN 1.5 §10.3.2.9), after the implicit conversions of singleton lists and of dates to dates and times.

## 7. DMN export and import

- `tok export` writes one `.dmn` file per decision requirements graph of a 0.7 document, named after its first decision (`limit.dmn`), with the DMN types of the document and its DMNDI diagram, and its picture `limit.drd.svg`; documents of earlier versions keep one DMN 1.3 file per decision table. Camunda 8 deploys the DMN 1.5 files; a business rule task names any decision of a graph (`decision=ID`).
- `tok import FILE.dmn [-o OUT.tok]` converts a DMN 1.1–1.5 file to a 0.7 document: decisions with their logic, input data, business knowledge models, knowledge sources, requirements, item definitions. The DMN 1.5 boxed expressions `conditional`, `filter`, `for`, `some` and `every` are imported as the equivalent literal expressions. Elements tok does not keep (decision services, imports of other models, text annotations and associations, organisation units, performance indicators, the diagram) are reported on standard error; the diagram is laid out again by tok.

## 8. Checks

| Code | Severity | Check |
|---|---|---|
| `R076` | error | the requirements form a cycle |
| `R077` | error | a decision, business knowledge model, entry or element has more than one form of logic (rows, lines, `expr=`, `invoke=`); `with=` without `invoke=`; `hit=`, `aggregation=` or `default=` without a decision table; `result` not last in its context; element lines mixed with entry and result lines |
| `R078` | error | `with=` names a parameter the business knowledge model does not have; malformed `params=` or `with=` |
| `R079` | warning | a decision or business knowledge model uses the name of a decision, input data or business knowledge model it does not require |
| `R080` | error | two elements of the graph, two entries of one context or two parameters of one function have the same name |
| `R081` | error | an unknown type; an `item` that is both a BPMN and a DMN type (`type=` with `structure=` or `kind=`) |
| `D006` | error | `hit=priority` or `hit=output-order` without output values |
| `D007` | error | `aggregation=` without `hit=collect`, with an unknown value, or on a table with several outputs |
| `D008` | error | a relation row has a different number of cells than its header |
| `D009` | error | a header cell of a 0.7 table that is not `name: type` or `name: type = values` |
| `C130` | error | a business knowledge model whose logic is an invocation (Camunda 8.10 does not execute it) |
| `C131` | error | a function definition whose body is not a literal expression (Camunda 8.10 executes only literal bodies) |
| `C132` | error | a decision without decision logic in an executable document (Camunda 8.10 rejects the DMN file) |

The references of `requires=` and `invoke=` are checked like other references (`R002`, `R003`). `C130`–`C132` are reported for executable documents with `target=camunda`, the default. Camunda 8.10 executes decision requirements graphs, decision tables, literal expressions, contexts, relations, lists, invocations and business knowledge models through its DMN engine; everything else of this design is descriptive for Camunda or evaluated by tok only.

## 9. Conformance

Conformance cases 53 and following cover every section above (valid documents and error cases); their expected outputs are produced by the Rust engine. The DMN TCK (Technology Compatibility Kit of the DMN community, Apache-2.0) is the conformance gate of FEEL and of decision evaluation: the compliance-level-2 and level-3 test cases the implementation supports are kept in `conformance/dmn-tck`, the test `rust/tok-core/tests/dmn_tck.rs` imports each model, evaluates each test case and compares the results; a baseline lists the cases expected to fail with a reason, and the test fails on any new failure. `dmn-tck.md` publishes the pass counts per level. The Julia core parses, prints and exports the new syntax at the core level.

## Decisions taken during implementation

1. **The new statements need a `tok 0.7` header.** Their FEEL is full FEEL and their numbers have 34 digits, which earlier documents do not have; in a document of an earlier version `inputdata`, `bkm`, `source`, `entry`, `result` and `element` stay unknown keywords (`P001`), the new attributes stay unknown attributes (`R010`) and the new hit policies unknown (`D003`), exactly as before. Every output of an older document is unchanged: its text, diagnostics, BPMN, DMN 1.3 files, `tok eval` results and simulation (the conformance cases 01–52, the MIWG gate and the parity tests confirm it).
2. **DMN 1.5 for 0.7 documents, DMN 1.3 for the others.** Earlier documents keep the DMN 1.3 export they had, so their files stay byte for byte the same. Documents of 0.7 export DMN 1.5 (`https://www.omg.org/spec/DMN/20230324/MODEL/`, DMNDI `https://www.omg.org/spec/DMN/20230324/DMNDI/`), whose namespace the DMN model of Camunda 8 reads; `exporterVersion` is the language version of the document, as in BPMN.
3. **Names are identifiers.** A tok label may contain anything (`"Czy zablokować konto?"`), so FEEL refers to an element by its identifier, or by `name=` when the name of a DMN element is not a tok identifier. The export writes the FEEL name as the DMN `name` (also the name of the `variable`, as DMN requires) and the label as `label` when it differs; Camunda 8 resolves required decisions by identifier and variable name, which agree.
4. **One requirement attribute.** DMN distinguishes information, knowledge and authority requirements by the kind of the required element, so one `requires=` suffices; the export groups the requirements in the order DMN 1.5 prescribes (information, knowledge, authority).
5. **Decision values.** In a 0.7 document the value of a decision table with one output is the value of that output (DMN 1.5 §8.2.11), not the object of outputs that `tok eval` gives for earlier documents; a business rule task stores it, as Camunda stores the result of such a table. The default result variable of a rule task is the only output of a decision table, otherwise the FEEL name of the decision.
6. **Default outputs.** DMN decision tables have default output entries, which the TCK uses, so `default=` was added. With `collect`, `rule-order` and `output-order` the default outputs form a list of one result; with an aggregation they are the result.
7. **Output values of priority.** DMN orders the matching rules by the output values of the outputs that have them; an output without output values does not contribute. `D006` is therefore reported only when no output has output values, and the allowed values of the type of an output count as its output values.
8. **Allowed values** are unary tests: a syntax error in `allowed=` is `F002` like other FEEL, not `R081`.
9. **Decimal semantics.** Numbers of 0.7 documents are decimal with 34 significant digits and rounding half even, as DMN 1.5 requires (IEEE 754 decimal128); the exponent range is that of the engine's decimal arithmetic (±999999), larger than that of decimal128. `sqrt`, `exp`, `log` and non-integral powers are computed with 44 digits and rounded. Earlier documents keep the 28 digits of tok 0.1–0.6, so none of their results change.
10. **Time zones.** tok includes no time zone database: offsets, `Z`, `UTC` and `Etc/UTC` are understood; other zone identifiers of known areas are kept, printed and compared by name, and arithmetic or comparison between a zone identifier and an offset is `null`. Following the TCK, dates and times compare equal at the resolution of a second, a date counts as the date and time at midnight UTC in arithmetic, and `years and months duration` counts the whole months between the dates (the time of day does not count).
11. **Name resolution.** A FEEL name is the longest sequence of words and of the symbols `. / - ' + *` that is a name of the document (an element of the graph, an entry, a parameter, a component of a DMN type, an output or relation column, a named argument of a built-in function) or of a built-in function; otherwise it is a single word. A name with dots that is not a variable reads the fields of the variable it starts with (`Person.Gender` is the field `Gender` of `Person`).
12. **Implicit conversions.** A list of one value is converted to its element (one level only), a value to a list of one, and a date to a date and time, where a type or a parameter needs it (DMN 1.5 §10.3.2.9.4). The built-in functions take the element of a list of one for parameters that need one value, and a single value as a list of one for parameters that need a list.
13. **Conformance of types.** `instance of` checks the structure of a value (every component of a DMN type present, allowed values not checked, as the TCK expects); the conversion to the type of an element also checks the allowed values. A value that does not conform is `null`.
14. **Quantifiers and ranges.** `some` and `every` are `null` when their condition is not a boolean for an element; a range with a `null` end is a range whose tests are `null`; a range whose low end is greater than its high end is `null`; `range("…")` accepts only ranges with literal ends (numbers, strings, temporal literals and the conversion functions applied to a string).
15. **Boxed conditional, filter and iterators** of DMN 1.5 are imported as literal expressions. A conditional becomes `if (c) then (a) else if (c) = false then (b) else null`, which is `null` for a condition that is not a boolean, as the boxed conditional is. A boxed filter whose match is not a boolean is `null` in DMN but drops the element as a FEEL filter does (two TCK cases).
16. **Function definitions as decision logic.** The logic of a decision or business knowledge model that is itself a function definition is written as a context whose only line is a `result` with `params=`; its value is the same function.
17. **Decision services, imports of other models, Java and PMML functions** are outside the language; the import reports them, and the TCK folders that need them are not part of the gate.
18. **The diagram.** The DMN export lays out each graph in rows: an element stands one row below the lowest element that requires it, and each element moves towards the mean position of the elements it is connected with, keeping the order of its row and a gap of 40 units; requirement edges are straight lines from the top of the required element to the bottom of the requiring one. The import does not keep a DMN diagram (tok has no coordinates) and reports it. `tok export` also writes the picture of each graph as `NAME.drd.svg`.
19. **Files.** A graph is a connected set of elements; its file is named after its first decision in document order (after its first element when it has no decision) and contains all DMN types of the document. A 0.7 document without a process writes no BPMN file.
20. **FEEL of process expressions.** In a 0.7 document conditions, scripts and the expressions of the executable layer are FEEL of level 3 as well, checked by `tok check` and evaluated by the simulator; process variables are single words, so a process expression resolves only the names of built-in functions.
21. **Camunda codes.** Camunda 8.10 (its DMN engine) evaluates decision tables, literal expressions, contexts, relations, lists and invocations as the logic of decisions, the same without invocations as the logic of business knowledge models, and only literal bodies of function definitions; it rejects a decision without logic. Hence `C130` (a business knowledge model with an invocation), `C131` (a function definition whose body is not literal) and `C132` (a decision without logic), all errors in executable documents for Camunda. Knowledge sources and DMN types need no code: Camunda ignores them.
22. **Checks of requirements.** `R079` is a warning, since the omission is a modelling slip that evaluation survives with `null`; it ignores names that a required element also has, and counts an invocation (`invoke=`) as a use of the business knowledge model it names.
23. **Conformance cases** of language 0.7 are numbered 53–60. Their expected outputs include the DMN file of every decision requirements graph (`expected/NAME.GRAPH.dmn`); `D008` and `D009`, found by the parser like `D002` and `D004`, belong to the core codes.
24. **DMN TCK results.** Of the selected test cases, 126 of 126 results of compliance level 2 and 3313 of 3331 results of level 3 pass (`dmn-tck.md`); the 18 that fail need a time zone database, unary tests of one value as range values, decision services, or the semantics of the boxed filter (decision 15).
25. **Decisions for Camunda.** A document is exported for Camunda when its header or a pool is `executable` and `target=camunda` (the default), as the BPMN export decides; a decision model without a process is executable with the header flag. Camunda 8.10 (feel-scala) reads names with spaces and the symbols `- + / * '` and `.` only in backticks, a form outside standard FEEL; the DMN export of such a document writes every name the FEEL parser resolves that is not a plain name (a letter or `_`, then letters, digits and `_`, non-ASCII letters included, which Camunda reads) in backticks, and such a key of a context literal as a string. Strings, built-in function names and the rest of the text stay as written; other documents keep standard FEEL. Every name tok accepts can be written this way, so no Camunda code is needed for it.
26. **Numbers as text.** DMN 1.5 gives FEEL numbers no scale (equal numbers are the same value) and converts them to their plain decimal form in its examples (`string(1.1) = "1.1"`); the TCK converts only integral numbers to text. tok writes the shortest form, `string(4 * 0.5) = "2"`; Camunda writes the scale of its Java decimal, `"2.0"`. tok follows the specification; the difference is documented in `feel.md` §9.5.
27. **Results on Camunda.** `conformance/camunda/run.py` deploys the DMN files of conformance case 61 to Camunda 8 Run 8.10.7 and compares Camunda's evaluation of nine decision inputs with `tok eval` and with the expected values: input data and decisions named with spaces, a dash and a non-ASCII letter, a decision table, a literal expression with a filter over a component named with two words, an invocation of a business knowledge model, a call of a business knowledge model inside a context, a context literal with keys of several words, and a list. All agree.

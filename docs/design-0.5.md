# Design: language 0.5 (BPMN 2.0.2 process modelling beyond Camunda)

Status: **accepted** (2026-10-27) and moved into `spec.md` (§4, §5.1.1, §5.2.1, §5.3, §5.4, §5.6, §6.3, §7, §7.1, §7.2, §10.4, §13–§15); the decisions taken during implementation are listed at the end. Language 0.5 (`tok 0.5` in the header) adds the elements of BPMN 2.0.2 process modelling that Camunda 8 does not execute. Documents that start with `tok 0.1` or `tok 0.3` remain valid and mean the same.

## Principles kept

1. Every statement starts with a keyword; one statement per line; blocks by indentation.
2. The simple syntax of descriptive models does not change. New elements are new keywords for new kinds of things (`transaction`, `complex`, `input`, `output`, `property`, `item`, `global`, `interface`, `key`) or attributes and flags of existing statements.
3. Everything in this document is available at the level `full`; the levels `basic` and `analytic` reject what is outside them (`R040`), as before.
4. In executable documents for Camunda (`target=camunda`, the default for executable documents) every element below that Camunda 8.10 does not execute is reported with a `C1xx` code naming the element; descriptive and analytic documents are not affected.

## 1. Transaction, cancel events and compensation

```
transaction rezerwacja "Rezerwacja pokoju i biletu"
  start s
  service pokoj "Rezerwacja pokoju" compensate=zwolnij-pokoj
  service zwolnij-pokoj "Zwolnienie pokoju" compensation
  xor ok "Czy udało się?"
  end gotowe
  end anuluj "Anulowanie" cancel
  flow s -> pokoj -> ok -> gotowe "tak" when="ok"
  flow ok -> anuluj "nie" default
boundary anulowano "Rezerwacja anulowana" on=rezerwacja cancel
```

| tok | BPMN |
|---|---|
| `transaction ID "Label"` + block | `transaction` (a sub-process; same block rules as `sub`) |
| `end … cancel` (flag) | end event with `cancelEventDefinition`; allowed only inside a transaction (`R060`) |
| `boundary … on=T cancel` (flag) | boundary event with `cancelEventDefinition`; `on=` must be a transaction (`R060`); always interrupting |
| `method=compensate\|store\|image` on `transaction` | `method` attribute (`##Compensate`, `##Store`, `##Image`); default compensate, printed only when given |

Compensation inside a transaction uses the 0.3 syntax (`compensate=`, `compensation`, `throw … compensate`).

## 2. Complex gateway

```
complex wybor "Czy zebrano dość recenzji?" activation="_activationCount >= 2"
```

`complex` → `complexGateway`; `activation="FEEL"` → `activationCondition`. Outgoing flows take `when=` and `default` like an inclusive gateway.

## 3. Standard loop

```
task proba "Próba połączenia" loop while="proby < 3" test-before loop-max=5
```

| tok | BPMN `standardLoopCharacteristics` |
|---|---|
| `loop` (flag, as in 0.1) | the element itself |
| `while="FEEL"` | `loopCondition` (the activity repeats while it is true) |
| `test-before` (flag) | `testBefore="true"` (default false: the activity runs at least once) |
| `loop-max=N` | `loopMaximum` |

`while`, `test-before` and `loop-max` require `loop` (`R061`).

## 4. Multiple and parallel multiple events

An event with two or more triggers is a multiple event:

```
start zgloszenie "Zgłoszenie e-mailem lub przez formularz" message="E-mail" message="Formularz"
catch potwierdzenie "Potwierdzenie i płatność" message="Potwierdzenie" signal="Wpłata" parallel
end koniec "Powiadomienia" message="Do klienta" signal="Zamknięto"
```

- A trigger attribute may now repeat, and different trigger attributes may be combined (§5.1 of the specification lists which triggers each event kind allows). Two or more triggers export one event definition each; on catching events (`start`, `catch`, `boundary`) BPMN reads them as "any one of them".
- `parallel` (flag) on a catching event with several triggers sets `parallelMultiple="true"`: all of them are needed (`R062` if the event has fewer than two triggers).
- `terminate`, `cancel`, `compensate` and `link` cannot be combined with other triggers (`R062`).

## 5. Instantiation

```
receive zamowienie "Przyjęcie zamówienia" message="Zamówienie" instantiate
eventgw start-gw "Co przyszło najpierw?" instantiate parallel
```

- `instantiate` on a `receive` task → `instantiate="true"`; the task has no incoming sequence flows and stands where a start event would (`R063`).
- `instantiate` on `eventgw` → `instantiate="true"`; with `parallel` → `eventGatewayType="Parallel"` (`R063`: no incoming sequence flows).

## 6. Process data

```
item wniosek-typ "Wniosek" structure="Wniosek"
input wniosek "Wniosek o zakup" type=wniosek-typ
output decyzja "Decyzja"
property licznik "Licznik prób" on=proba
data skan "Skan wniosku" type=wniosek-typ
reads ocena wniosek
writes ocena decyzja
```

| Statement | BPMN |
|---|---|
| `item ID ["Label"] [structure="…"] [kind=information\|physical] [collection]` | `itemDefinition` (top level of definitions) |
| `input ID ["Label"] [type=ITEM] [collection]` | `dataInput` of the process (or of the sub-process or activity named by `on=`), in its `ioSpecification`; one input set containing every input |
| `output ID ["Label"] [type=ITEM] [collection]` | `dataOutput`, likewise; one output set |
| `property ID ["Label"] [type=ITEM] [on=ID]` | `property` of the process, or of the activity or event named by `on=` |
| `type=ITEM` on `data`, `store`, `input`, `output`, `property` | `itemSubjectRef` |

- A process `input`/`output` is drawn as a data input/output shape; `reads` and `writes` accept inputs, outputs and properties as well as data objects and stores.
- An activity that `reads` or `writes` data gets the `ioSpecification` BPMN requires for its associations automatically (data inputs and outputs named after the data elements); `input … on=ACTIVITY` declares them explicitly.
- Data state transitions: data objects already carry `state=`; a transition is an activity that reads a data object in one state and writes it in another. The checker warns when an activity reads and writes the same data object with the same state (`R064`).

## 7. Resources and performers

```
resource bibliotekarz "Bibliotekarz" count=2
user wypozycz "Wypożyczenie" by=bibliotekarz
```

`resource` (already used by the simulation) now also exports a BPMN `resource`; `by=` exports `performer` (`humanPerformer` on `user` and `manual` tasks) with `resourceRef`. `count=` stays a simulation setting.

## 8. Global tasks, interfaces and operations

```
global sprawdzenie "Sprawdzenie tożsamości" kind=user
call sprawdz "Sprawdzenie tożsamości" task=sprawdzenie

interface katalog "Katalog biblioteczny"
  operation szukaj "Wyszukanie rekordu" in="Zapytanie" out="Rekord" error="Brak rekordu"
service wyszukaj "Wyszukanie w katalogu" operation=szukaj
```

- `global ID ["Label"] kind=task|user|manual|script|rule` → `globalTask`, `globalUserTask` … at the top level; `call … task=ID` → `calledElement` pointing to it (`R065`: `process=` and `task=` are exclusive).
- `interface ID ["Label"]` with `operation ID ["Label"] in="Message" [out="Message"] [error="Error" …]` lines → `interface`/`operation` with `inMessageRef`, `outMessageRef`, `errorRef`; `operation=ID` on `service`, `send` and `receive` tasks → `operationRef`. Processes that use an operation list its interface in `supportedInterfaceRef`.

## 9. Correlation

```
key zamowienie "Numer zamówienia"
  prop numer message="Zamówienie" path="zamowienie.numer"
  prop numer message="Potwierdzenie" path="potwierdzenie.nr"
```

`key ID ["Label"]` → `correlationKey` of the collaboration (or of the process when there are no pools); each distinct `prop` name → a `correlationProperty` with one `correlationPropertyRetrievalExpression` per message (`messagePath` with the `path` text). Message names are those of the document (§5.1).

## 10. Checker

New codes: `R060`–`R066` (above, and `R066` for references of `type=`, `task=`, `operation=` to undeclared items), `C100`–`C110` for elements Camunda 8.10 does not execute (transaction, cancel events, complex gateway, standard loop with condition, multiple and parallel multiple events, instantiating receive task, instantiating event-based gateway, global tasks as called elements, operations, correlation keys). Every rule of the BPMN 2.0.2 specification that the checker enforces is listed with its clause in `docs/bpmn-rules.md`.

## 11. Simulation

Transactions run like sub-processes; a cancel end event runs the compensation handlers of the completed activities of the transaction and then leaves through the cancel boundary event. A complex gateway passes the branches whose conditions hold and waits until its activation condition holds for the arriving tokens (`_activationCount`). A standard loop repeats while `while` holds, at most `loop-max` times, testing before or after as declared. A catching multiple event fires on the first of its triggers (all of them with `parallel`); timers among them use their durations. An instantiating receive task or event-based gateway starts the case like a start event.

## 12. Conformance

Cases 33 and following cover every section above; the expected outputs are produced by the Rust engine (the pure-Python implementation stays at language 0.3 and checks cases 01–32). The models of the BPMN Model Interchange Working Group (MIWG) test suite are imported, exported and imported again; the coverage table `docs/miwg.md` lists, per reference model, whether every element and attribute survives.

## Decisions taken during implementation

Where the design was ambiguous or did not match BPMN 2.0.2, the BPMN-correct option was chosen:

1. **Older headers keep their meaning.** A document that declares `tok 0.1` or `tok 0.3` reads a repeated key as before (the last value counts) and reports two triggers as `R012`; only a `tok 0.5` document has multiple events. Likewise `resource` and `by=` export BPMN resources and performers only in `tok 0.5` documents, so the BPMN of every 0.1 and 0.3 document stays byte for byte the same. All other elements of 0.5 may be used under any header.
2. **`structure=` is a qualified name.** `structureRef` is an `xsd:QName`, so `wniosek.xsd#Wniosek` (the first example of §6) is not valid BPMN. `structure=` takes a type name without a prefix (`Wniosek`) or an XML Schema type (`xsd:string`, for which the exporter declares the prefix); other values are `R013`. The import keeps XML Schema types and drops other prefixes with a warning.
3. **Item labels.** `itemDefinition` has no `name` attribute in BPMN 2.0.2; the label of `item` is exported as its `documentation` and read back from it.
4. **Associations of `reads` and `writes`.** In `tok 0.5` documents every association connects the data element with a generated data input (`ACTIVITY-in-N`) or data output (`ACTIVITY-out-N`) of the activity's `ioSpecification` (BPMN 2.0.2 §10.4), merged with the inputs and outputs declared by `input … on=ACTIVITY`. Documents of 0.1 and 0.3 keep the `__targetRef_placeholder` property of Camunda Modeler (valid by the schema, since a property is an item-aware element).
5. **Correlation keys need pools.** `tProcess` has no `correlationKey`; BPMN 2.0.2 keeps correlation keys in collaborations only. `key` in a document without pools is `R013` instead of being exported "to the process". Correlation property identifiers are `CorrelationProperty_NAME`, shared by the keys that use a name.
6. **`R012` and `R062`.** In 0.5 documents `R062` replaces `R012`: it also reports the catch-all flags `error`/`escalation` combined with other triggers, `parallel` on a throwing event, and a repeated key on an element that is not an event.
7. **Further checks.** `R060` also covers `method=` values and `cancel` on events other than end and boundary events; `R061` also requires `loop-max` to be a whole number; `R063` also requires `instantiate` to stand in a process (not a sub-process), `parallel` on `eventgw` to come with `instantiate`, and reports `instantiate` on other activities; `R065` also covers `kind=` of `global`; an `operation` needs `in=` and a `prop` needs `message=` and `path=` (`R013`). `R064` applies only to data objects with `state=`, compared by label.
8. **Camunda codes.** `C100`–`C108` are errors; `C109` (operations) and `C110` (correlation keys) are warnings, because Camunda 8.10 ignores those elements instead of rejecting the model. `C103` is reported only for a standard loop with `while=`, `test-before` or `loop-max=`, so documents of 0.3 with a plain `loop` do not change.
9. **Canonical order.** `item` and `global` follow the header, then each `interface`; each `key` follows the message flows; `input`, `output` and `property` are printed with the data of their block.
10. **MIWG.** The import reads the MIWG reference models without errors (`conformance/miwg`), but the coverage table `docs/miwg.md` of §12 is not part of this release.

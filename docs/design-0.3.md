# Design proposal: language 0.3 (complete Camunda 8 coverage)

Status: **accepted** (2026-10-27) and moved into `spec.md` §5.3 and §10.3. Decision on review: everything in this document is an optional executable layer; the simple syntax of descriptive and analytic models is unchanged, and the levels `basic` and `analytic` reject these attributes. Outside the executable layer, compatibility with Camunda means the elements of Camunda Modeler and the rules for connecting and organizing them.

Version 0.3 of the tools introduces version **0.3 of the language** (`tok 0.3` in the header). From now on the language version follows the minor version of the release that changes it; documents that start with `tok 0.1` remain valid and mean the same.

## Principles kept

1. Every statement starts with a keyword; one statement per line.
2. New features are **attributes and flags** of existing statements wherever possible. The only new keyword is `adhoc`.
3. Attribute names are lowercase words joined by `-`.
4. Lists and pairs inside one attribute are written in a quoted string and separated by `;` (pairs) or `,` (plain lists), as `assign=` and `options=` already do.

### Expressions and static values

Two kinds of attribute values exist and the spec lists which is which:

| Kind | Attributes | Written as | Exported as |
|---|---|---|---|
| **always an expression** | `when`, `expr`, `condition`, `correlation`, `collection`, `output-item`, `until`, `active`, the right-hand side of `inputs`/`outputs` | FEEL without `=`: `when="cena > 150"` | `=cena > 150` |
| **static or expression** | `assignee`, `candidate-users`, `candidate-groups`, `due`, `follow-up`, `priority`, `process`, `type`, `timer`, `business-id` | a static value as is: `assignee="kierownik"`; an expression starts with `=`: `assignee="=wniosek.przelozony"` | unchanged (as Camunda Modeler writes them) |

This follows the convention users already know from Camunda Modeler for the second group, and keeps the shorter form for attributes that are always expressions.

## 1. Multi-instance in full

```
user ocena "Ocena recenzji" parallel collection="recenzje" item=recenzja output=oceny output-item="ocena" until="numberOfCompletedInstances >= 2"
```

| tok | Camunda 8 / BPMN |
|---|---|
| `parallel` / `sequential` | `multiInstanceLoopCharacteristics isSequential` |
| `collection="expr"` | `zeebe:loopCharacteristics inputCollection="=expr"` |
| `item=name` | `inputElement="name"` |
| `output=name` | `outputCollection="name"` |
| `output-item="expr"` | `outputElement="=expr"` |
| `until="expr"` | `completionCondition` `=expr` |

Checker: `collection` without `parallel`/`sequential` is an error; `output` requires `output-item` and vice versa.

## 2. Ad-hoc sub-process (new keyword `adhoc`)

```
adhoc opracowanie "Opracowanie spuścizny" active="wybrane" until="gotowe = true" wait
  user inwentarz "Sporządzenie inwentarza"
  user digitalizacja "Digitalizacja wybranych jednostek"
  manual konserwacja "Konserwacja"
```

| tok | Camunda 8 / BPMN |
|---|---|
| `adhoc ID "Label"` + block | `adHocSubProcess` |
| `active="expr"` | `zeebe:adHoc activeElementsCollection="=expr"` (list of inner element ids) |
| `until="expr"` | `completionCondition` |
| `wait` (flag) | `cancelRemainingInstances="false"` (default: remaining elements are cancelled) |
| `sequential` (flag, BPMN only) | `ordering="Sequential"`; reported by `--target camunda` |

Checker: the block contains at least one activity and no start or end events (`R050`); flows between inner activities are allowed.

## 3. Call activity

```
call wysylka "Wysyłka zamówienia" process=wysylka binding=version-tag version="v2" child-vars=none outputs="numer = numerPrzesylki"
```

| tok | Camunda 8 |
|---|---|
| `process=ID` (static or `=expr`) | `zeebe:calledElement processId` |
| `binding=latest\|deployment\|version-tag`, `version="tag"` | `bindingType`, `versionTag` |
| `child-vars=all\|none`, `parent-vars=all\|none` | `propagateAllChildVariables`, `propagateAllParentVariables` (default `all`) |
| `business-id="…"` | `businessId` |

`binding=` and `version=` are also accepted on `rule` tasks (decision binding) and on elements with `form=` (form binding).

## 4. Variable mappings

```
service rejestracja "Rejestracja zgłoszenia" type="rejestracja" inputs="numer = zgloszenie.id; pilne = priorytet > 70" outputs="sygnatura = wynik.sygnatura"
```

`inputs` and `outputs` hold pairs `target = expression`, separated by `;`. The target is a variable or a nested property (`nadawca.iban`); the expression is FEEL without `=`. Allowed on activities, sub-processes, call activities and on catch, boundary and start events (outputs only), as Camunda allows. Exported as `zeebe:ioMapping` with `zeebe:input`/`zeebe:output source="=expr" target="name"`.

## 5. User tasks

```
user akceptacja "Akceptacja wniosku" form=f-akceptacja assignee="=wniosek.przelozony" candidate-groups="kierownicy, sekretariat" due="=now() + duration(\"P3D\")" priority=70
```

| tok | Camunda 8 |
|---|---|
| `assignee`, `candidate-users`, `candidate-groups` | `zeebe:assignmentDefinition` (lists comma-separated when static) |
| `due`, `follow-up` | `zeebe:taskSchedule dueDate`, `followUpDate` (ISO 8601 with zone, or `=expr`) |
| `priority` (0–100) | `zeebe:priorityDefinition priority` |

## 6. Jobs: service and send tasks, message throw and end events

```
service powiadomienie "Wysłanie powiadomienia" type="email" retries=5 headers="szablon: odmowa; jezyk: pl"
end wyslano "Odpowiedź wysłana" message="Odpowiedź" type="wysylka-odpowiedzi"
```

`type`, `retries` and `headers` (pairs `key: value` separated by `;`) map to `zeebe:taskDefinition` and `zeebe:taskHeaders`. Message throw and end events in executable documents need `type=` (Camunda executes them as jobs) — checker code `C022`.

### Connectors (typed sugar)

```
service katalog "Pobranie rekordu z katalogu" connector=rest method=GET url="=\"https://api.example.org/rekord/\" + id" result=rekord
```

`connector=rest` expands to the Camunda REST outbound connector (job type `io.camunda:http-json:1`, inputs `method`, `url`, `authentication.type = noAuth`, optional `headers-in`, `query`, `body`, and the header `resultVariable`). The exact input names are verified against Camunda 8 Run before release; other connectors are reachable through `type=`, `inputs=` and `headers=`.

## 7. Compensation

```
service rezerwacja "Rezerwacja środków" compensate=zwolnienie
service zwolnienie "Zwolnienie rezerwacji" compensation
throw cofniecie "Cofnięcie rezerwacji" compensate activity=rezerwacja
```

| tok | BPMN / Camunda 8 |
|---|---|
| `compensate=HANDLER` on an activity | boundary event with `compensateEventDefinition` attached to the activity + association to the handler |
| `compensation` flag on the handler | `isForCompensation="true"` (no sequence flows, `R051`) |
| `throw … compensate [activity=ID]` / `end … compensate [activity=ID]` | `compensateEventDefinition activityRef` |

## 8. Errors and escalations

- `error` and `escalation` as **flags** (without a name) on boundary events and event sub-process starts catch every error or escalation.
- `code="…"` keeps its meaning. The proposed `code-var=`/`message-var=` were dropped after testing on Camunda 8 Run 8.10.7: the engine accepts `errorCodeVariable`/`errorMessageVariable` in the XML but ignores them, and neither `errorCode` nor `error.code` is available to output mappings of the catching event. Variables sent with the error are merged with `outputs=` on the catching event.

## 9. Conditional events

`condition="expr"` as today, plus `events="create, update"` (Camunda `zeebe:conditionalFilter variableEvents`) on intermediate, boundary and event sub-process start events. Root-level conditional starts take no `events=` (`R052`).

## 10. Listeners and process version

- `listeners="start: typ-a; end: typ-b"` on any element (execution listeners); on user tasks the events `creating`, `assigning`, `updating`, `completing`, `canceling` define task listeners. Each entry may add `retries` as `start: typ-a (5)`.
- `version-tag="v1"` on the header (document without pools) or on a pool → `zeebe:versionTag` of the process.

## 11. Checker: `--target camunda`

All Camunda-specific rules are grouped under `tok check --target camunda` (default for executable documents). Elements of BPMN 2.0.2 that Camunda 8.10 does not execute — introduced in 0.5 — are reported there with the Camunda version the rule refers to.

## 12. New diagnostic codes

| Code | Meaning |
|---|---|
| R050 | ad-hoc sub-process without activities, or with start/end events |
| R051 | compensation handler with sequence flows; `compensate=` pointing to an activity without the `compensation` flag |
| R052 | `events=` on a root-level conditional start event |
| R053 | malformed `inputs`/`outputs`/`headers`/`listeners` pair list |
| R054 | `collection` without `parallel`/`sequential`; `output` without `output-item` |
| C022 | message throw/end event without `type=` in an executable document |
| C090 | static value of `due`/`follow-up` not ISO 8601 with a zone |
| C091 | `priority` outside 0–100 |

## 13. Acceptance for 0.3

- Spec updated; Python, Rust (tok-core, tok-bpmn validator rules for the new elements) and the Julia core implement it; conformance corpus extended with one case per section above.
- Every executable feature is deployed to Camunda 8 Run 8.10 and an instance is driven to its expected end event by a script (`conformance/camunda/`).
- `tok import` reads every construct back (round trip), the course skill and cheat sheets are updated.

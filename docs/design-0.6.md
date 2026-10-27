# Design: language 0.6 (collaboration, choreography, diagram interchange)

Status: **accepted** (2026-10-27); the content moves into `spec.md` with the release. Language 0.6 (`tok 0.6` in the header) adds conversations, choreographies and participant multiplicity; the tools add a layout sidecar for round trips with modelling tools and report the BPMN conformance classes. Documents for 0.1, 0.3 and 0.5 remain valid and mean the same.

## Principles kept

Keyword-first lines, blocks by indentation, new keywords only for new kinds of things, attributes and flags otherwise; the simple syntax of descriptive models is unchanged. The new statements are available at the level `full`. Camunda 8.10 executes none of them: executable documents for Camunda report them (`C120`–`C123`).

## 1. Conversations

```
pool czytelnik "Czytelnik"
pool biblioteka "Biblioteka"
pool hurtownia "Hurtownia"

conversation wypozyczenie "Wypożyczenie" pools="czytelnik biblioteka" messages="m-zamowienie m-odbior"
subconversation zakup "Zakup książek" pools="biblioteka hurtownia"
  conversation oferta "Oferta" pools="biblioteka hurtownia"
  conversation zamowienie "Zamówienie" pools="biblioteka hurtownia"
callconversation reklamacja "Reklamacja" pools="biblioteka hurtownia" collaboration=reklamacje
```

| tok | BPMN |
|---|---|
| `conversation ID ["Label"] pools="P P …" [messages="F F …"]` | `conversation` with `participantRef`s and `messageFlowRef`s; one `conversationLink` per pool |
| `subconversation ID ["Label"] pools="…"` + block of conversation statements | `subConversation` containing the inner conversation nodes |
| `callconversation ID ["Label"] pools="…" collaboration=ID` | `callConversation` with `calledCollaborationRef` |

- `pools=` names at least two pools of the document (`R070`); `messages=` names message flows (§6.2 of the specification) between those pools (`R071`). Message flows get an explicit `id=` when a conversation refers to them.
- Conversation nodes are drawn between the pools they link (hexagons, a double border for calls, a `+` marker for sub-conversations), with conversation links as double lines.

## 2. Choreographies

```
pool klient "Klient"
pool sklep "Sklep"

choreography zakupy "Zakupy w sklepie"
  start s
  choreo zamow "Złożenie zamówienia" from=klient to=sklep message="Zamówienie" reply="Potwierdzenie"
  xor dostepne "Czy towar dostępny?"
  choreo wyslij "Wysyłka" from=sklep to=klient message="Paczka"
  choreo odmowa "Odmowa" from=sklep to=klient message="Brak towaru"
  end e1
  end e2
  flow s -> zamow -> dostepne
  flow dostepne -> wyslij "tak" when="dostepne"
  flow dostepne -> odmowa "nie" default
  flow wyslij -> e1
  flow odmowa -> e2
```

| tok | BPMN |
|---|---|
| `choreography ID ["Label"]` + block | `choreography` (a separate root element; its flow nodes, gateways and sequence flows as in a process) |
| `choreo ID ["Label"] from=P to=P [message="M"] [reply="M"]` | `choreographyTask` with two `participantRef`s, `initiatingParticipantRef` = `from`, and one `messageFlow` (`from` → `to`, `messageRef`) plus one for `reply` (`to` → `from`) |
| `subchoreo ID ["Label"] pools="P P …" from=P` + block | `subChoreography` with its participants and initiating participant |
| `callchoreo ID ["Label"] pools="…" from=P choreography=ID` | `callChoreography` with `calledChoreographyRef` |
| `loop`, `parallel`, `sequential` on choreography activities | `loopType` `Standard`, `MultiInstanceParallel`, `MultiInstanceSequential` |

- Participants of a choreography are pools of the document, usually black-box pools (`pool ID "Label"` without a block); `from`/`to` must differ and be pools (`R072`).
- Inside a choreography only events without triggers or with timer, conditional or signal triggers, `xor`, `and`, `eventgw` and `or` gateways, and choreography activities are allowed (`R073`).
- Choreography tasks are drawn as rounded rectangles with participant bands (the initiating participant white, the other one shaded) and an envelope for each message.

## 3. Participant multiplicity

```
pool recenzent "Recenzent" min=2 max=5
```

`min=`/`max=` → `participantMultiplicity` (`minimum`, `maximum`; default minimum 2 when only `max` is given, `R074` if `max` < `min` or `min` < 0); the pool is drawn with the multi-instance marker. Participant associations in calls of collaborations and choreographies are written `map="inner:outer; …"` on `callconversation` and `callchoreo`.

## 4. Diagram interchange round trip

- `tok import FILE.bpmn [-o FILE.tok]` writes the diagram of the imported file into a layout sidecar `FILE.tok.layout` next to the tok file: one line per shape `ID x y w h` and per edge `ID x1 y1 x2 y2 …` (integers, in the units of the file), plus label bounds as `ID/label x y w h`; a header line `tok-layout 1`.
- `tok bpmn`, `tok svg` and `tok export` read `FILE.tok.layout` when it exists next to the document: shapes and edges whose identifiers are still in the model and whose endpoints did not change keep their geometry; new or changed elements are placed by the automatic layout around them, and edges attached to moved shapes are rerouted. `--no-sidecar` ignores the file. `tok import --no-layout` does not write it.
- The sidecar is plain text so that it can be kept in version control next to the model; it never changes the meaning of the model.

## 5. BPMN conformance classes

`tok check --class descriptive|analytic|common-executable|full` checks the document against the element sets of the BPMN 2.0.2 process modelling conformance sub-classes (specification §2.1.1–2.1.3): every element or attribute outside the chosen class is reported as `R075` with the class that would contain it. `tok check --json` always reports the smallest class the document fits (`"bpmn_class": "descriptive"` …). The existing tok levels (`basic`, `analytic`, `full`) are unchanged; the table in `docs/bpmn-rules.md` relates them to the BPMN classes.

## 6. Simulation

Conversations and choreographies are not simulated (they describe interactions, not the work of one participant); `tok sim` ignores them and simulates the processes of the document as before. A pool with `min`/`max` is simulated once per case.

## 7. Conformance

Cases 44 and following cover each section; the sidecar has its own tests (import of the course diagrams `conformance/import/*.bpmn` and of the MIWG models, export, comparison of the geometry with the original diagram: identical for unchanged elements).

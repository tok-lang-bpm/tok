# Design: language 0.6 (collaboration, choreography, diagram interchange)

Status: **implemented** (2026-10-28); the language is specified in `spec.md` (§4, §5.5, §5.7, §6.4, §10.4, §11, §13–§16); the decisions taken during implementation are listed at the end. Language 0.6 (`tok 0.6` in the header) adds conversations, choreographies and participant multiplicity; the tools add a layout sidecar for round trips with modelling tools and report the BPMN conformance classes. Documents for 0.1, 0.3 and 0.5 remain valid and mean the same.

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

## Decisions taken during implementation

Where the design was ambiguous or did not match BPMN 2.0.2, the BPMN-correct option was chosen:

1. **Older headers keep their meaning.** The new statements (`conversation`, `subconversation`, `callconversation`, `choreography` and its activities) were unknown keywords before, so they may be used under any header. `min=` and `max=` on a pool were unknown attributes (`R010`); they give a participant multiplicity only in a `tok 0.6` document, so the BPMN of every older document stays byte for byte the same. The import prints a file that uses any of it as `tok 0.6`.
2. **Choreography events and gateways follow BPMN 2.0.2 §11.6 and §11.7.** Besides the events listed in §2 the standard allows a terminate end event, link events, an intermediate event without a trigger, boundary events with timer, conditional or signal triggers on choreography activities, and the complex gateway; tok accepts them. A message trigger (BPMN shows the sender of a message by a choreography task, not by an event), throwing signals, errors, escalations, compensation and cancel stay `R073`, as do activities of processes and a choreography without flow nodes.
3. **Participants of a choreography.** A choreography is a collaboration (`tChoreography` extends `tCollaboration`) and refers to its own participants. Each choreography exports one `participant` per pool its activities name (identifier `CHOREOGRAPHY-POOL`, with the name and the multiplicity of the pool); the pools stay participants of the collaboration. The import maps a participant of a choreography back to the pool whose identifier follows the prefix, else to the only pool with its name, else to a new black-box pool.
4. **Message flows of choreography tasks.** BPMN requires at least one `messageFlowRef` on a choreography task. The flow from `from` to `to` is therefore always exported (`TASK-message`, with `messageRef` only when `message=` is given); `reply=` adds the flow back (`TASK-reply`; `reply=""` is a reply without a message name). The import reports message flows of a task whose identifiers differ from these.
5. **Sequence flows of a choreography** may be written anywhere, like those of processes (their container is given by their ends, `R021`); the canonical form prints them at the end of the block of their choreography. Choreographies follow the pools, conversation nodes follow the message flows.
6. **Conversation links.** `pools=` gives the participants of a conversation node; every top-level node gets one `conversationLink` per pool (`NODE-link-POOL`). The nodes inside a sub-conversation have no links and are not drawn. The import takes the pools of a node from its participant references and from the links that end at it, and reports links whose identifiers differ.
7. **`messages=`** is allowed on all three conversation statements (every BPMN conversation node has `messageFlowRef`). A message flow that a conversation names is printed with `id=`, and the import keeps its identifier also with `--no-flow-ids`.
8. **`collaboration=`** of `callconversation` names a choreography of the document (a choreography is a collaboration) or a collaboration of another file, written as it is. With a choreography of the document, the inner participants of `map=` are its pools; the inner participants of `callchoreo … map=` are pools of the called choreography (`R072` otherwise). The outer participants are pools of `pools=` (`R070`, `R072`). On other statements `map=` is an unknown attribute (`R010`).
9. **Multiplicity.** The design gives the minimum 2 when only `max=` is given; BPMN 2.0.2 (`ParticipantMultiplicity`, §9.2) gives the maximum 1 when it is absent, so `min=` alone allows only 0 or 1. `R074` reports values that are not whole numbers, `min < 0`, `max < 1` and `max < min`. BPMN shows the multi-instance marker of a participant when its maximum is greater than 1; tok draws it then. The import writes `min=` only when it is not 2 or there is no maximum.
10. **Camunda codes.** Camunda 8.10 ignores conversations and multiplicity and does not execute choreographies: `C120` (conversation and sub-conversation), `C121` (call conversation) and `C123` (multiplicity of a pool) are warnings, `C122` (choreography) is an error. The checks of the executable profile (`C011`–`C091`, `C100`–`C110`) do not apply to the flow nodes of a choreography.
11. **Levels.** `basic` and `analytic` report each conversation node, each choreography (once, not its content) and `min=`/`max=` of a pool (`R040`).
12. **BPMN conformance classes.** The element sets are those of BPMN 2.0.2 clause 2 (Tables 2.1–2.4: Descriptive, Analytic, Common Executable). The Common Executable class is not a superset of the Analytic one (it has no pools, lanes, message flows, send and receive tasks, inclusive gateways or signal events, but has data inputs and outputs, item definitions, operations and resources), so the smallest class is chosen in the order descriptive, analytic, common-executable, full, and `R075` names the first class in this order that contains an element. `full` is the whole modelling language: full Process Modeling Conformance, which contains conversations, together with the choreographies of Choreography Modeling Conformance. Following the rule of clause 2 that an attribute a sub-class does not mention is not in it: a message flow with a label or `message=` (tok declares its message, `messageRef`) is Analytic; a condition or default flag counts only on flows leaving an exclusive or inclusive gateway (an inclusive default is Analytic only); `state=` (`dataState`) is outside the sub-classes; documentation belongs to Descriptive and Analytic but not to Common Executable; `while=` (`loopCondition`) and performers of user tasks are Common Executable. Execution attributes of Camunda, decisions, forms and simulation statements are not BPMN elements and are not classified. `tok check --json` always reports `"bpmn_class"`; the frozen pure-Python command does not, and its parity test ignores that field and the option `--class`.
13. **Drawing.** A conversation node stands in the gap below the first pool it links (the gap grows from 40 to 110 units), nearest to the middle where no message flow, label or other node is in the way, with its name on the right. Each choreography is laid out like a process in a `BPMNDiagram` of its own (`BPMNDiagram_ID`), below the collaboration and the choreographies before it, with its name above it in the picture. Choreography tasks are 110 units wide and 60 + 20 per participant high, rows are 90 units apart (room for the envelopes), expanded sub-choreographies have room for their bands, and boundary events of choreography activities stand at the bottom-left corner, away from the envelope of the reply. Participant bands are written as BPMN DI shapes of the participant (`choreographyActivityShape`, `participantBandKind`, `isMessageVisible`). The message flows of a choreography are shown by envelopes, not edges, so the diagram check `D02` does not ask for them.
14. **Conformance cases** of language 0.6 are numbered 47–52 (44–46 were taken by the additions of `design-0.5.md` §13).
15. **The sidecar is written next to the tok file**, so `tok import` writes it only with `-o` (`OUTPUT.layout`); printing to the standard output writes no file. `--no-layout` suppresses it.
16. **Sidecar lines.** A line with four numbers can be a shape or an edge with two waypoints; the element the line names decides. Coordinates are rounded to whole units, as the design specifies, so an unchanged element keeps its geometry up to that rounding. Bands of choreography activities are not stored: they follow from the bounds of their activity.
17. **Several diagrams in one file** (several collaborations, a sub-process drawn in a diagram of its own, choreographies): tok draws one picture. The first diagram keeps its coordinates; every further one moves below the one before it (by whole units) and keeps its own arrangement.
18. **Elements tok identifies differently** (data associations, associations of notes, flows with derived identifiers, generated compensation boundary events, notes renamed to avoid a clash) are matched by what they connect, so their geometry is kept too. Groups have no shape in the automatic layout; a group the sidecar names is drawn with its bounds.
19. **Placement around kept elements.** An edge keeps its waypoints when both ends kept their geometry and its first and last waypoints lie within 25 units of their bounds; otherwise it is routed again (orthogonally for sequence flows, vertically for message flows and conversation links, straight for associations). A new shape is placed at the distance the automatic layout puts it from a neighbour that already has its place (the host of a boundary event first, then the source of an incoming flow, the target of an outgoing one, other connections), moved to the right until it overlaps nothing; without such a neighbour, relative to its container; what is left goes below the diagram. A new pool, lane or sub-process surrounds its placed content. Kept pools and lanes grow to the right and kept sub-processes in both directions to hold new content.

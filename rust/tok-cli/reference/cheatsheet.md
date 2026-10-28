# tok 1.0 — cheat sheet

One statement per line. Blocks by indentation (spaces). `#` starts a comment.
Every statement starts with a keyword. Identifiers: `[A-Za-z_][A-Za-z0-9_-]*`, unique. Labels: "quoted text".
Attributes: `key=value` (value: word, number or "text") and bare flags. A key or flag is written once per line, the triggers of
multiple events aside (in 1.0 a repeated one is the deprecation warning `R090`).

```
tok 1.0 "Title" [executable]                       # header, first line (0.1–0.7 documents stay valid)
    [process-name="Name"] (process of a document without pools)   doc="Text" on any statement: documentation

start ID "Label" [message="M" | timer=P3D | signal="S" | condition="feel"] [form=F]
end ID "Label" [message="M" | error="E" | escalation="X" | signal="S" | terminate]   # error/escalation: [code="E1"]
catch ID "Label" message="M" | timer=PT2H | signal="S" | condition="feel" | link="L"
throw ID "Label" [message="M" | escalation="X" | signal="S" | link="L" | compensate]
boundary ID "Label" on=ACTIVITY timer=P7D [noninterrupting]   # also message/error/escalation/signal
    several triggers = multiple event: message="A" message="B" signal="C"; + parallel = all of them (catching)
    trigger not given yet: flags message timer signal condition error escalation (start ID "Label" message)

task | user | manual | service | script | rule | send | receive | call   ID "Label"
    flags: loop parallel sequential compensation
    user … form=F        rule … decision=D result=var        script … expr="feel" result=var
    service … type="job"  send/receive … message="M"          call … process=P | task=GLOBAL
    loop while="feel" [test-before] [loop-max=5]              receive … instantiate (starts the process)
    service|send|receive … operation=OP                       user … by=RESOURCE (performer)
sub ID "Label"           # embedded sub-process: indented block with its own start and end
adhoc ID "Label"         # ad-hoc sub-process: activities in any order, no start or end events
eventsub ID "Label"      # event sub-process: block starts with a start event with a trigger
transaction ID "Label" [method=compensate|store|image]   # block like sub; inside: end ID cancel
boundary ID on=TRANSACTION cancel                        # leaves the transaction after cancel

xor | and | or | eventgw | complex   ID ["Question?"]
complex ID "?" activation="_activationCount >= 2"        eventgw ID instantiate [parallel]

pool ID "Label" [process=P] [process-name="Name"] [executable]    # with a block: participant + process
  lane ID "Label"                           # lanes nest; nodes go inside lanes
pool ID "Label" [min=2] [max=5]             # no block: black-box pool; min/max: several participants (0.6)

flow A -> B -> C                            # chain of sequence flows
flow G -> D "tak" when="amount > 100"       # one flow: label, FEEL condition
flow G -> E "nie" default                   # default flow; when="" = condition not written yet
message A -> B "Label" [id=ID]              # message flow between pools (nodes or pools)

conversation ID "Label" pools="P Q" [messages="M1 M2"]       # conversation between pools (message flow ids)
subconversation ID "Label" pools="P Q"                       # block: conversation nodes
callconversation ID "Label" pools="P Q" collaboration=ID [map="inner:outer; …"]

choreography ID "Label"                     # block: start/end/catch/throw/boundary, xor and or eventgw complex, flows
  choreo ID "Label" from=P to=Q [message="M"] [reply="R"] [loop|parallel|sequential]
  subchoreo ID "Label" pools="P Q R" from=P   # block like a choreography
  callchoreo ID "Label" pools="P Q" from=P choreography=ID [map="inner:outer; …"]

data ID "Label" [state="złożony"] [type=ITEM] [object="Data object"] [collection]
store ID "Label" [type=ITEM]
reads ACTIVITY DATA [input=ID]   writes ACTIVITY DATA [output=ID]   # DATA: data, store, input, output, property
    reads END|THROW DATA   writes START|CATCH|BOUNDARY DATA          # data of events
item ID "Label" [structure=Name|xsd:string|p:Name namespace="URI"] [kind=information|physical] [collection]
input ID "Label" [type=ITEM] [collection] [on=ACTIVITY|EVENT]    output ID … (same)
property ID "Label" [type=ITEM] [on=ACTIVITY|EVENT]

global ID "Label" [kind=task|user|manual|script|rule]       # called by call … task=ID
interface ID "Label"
  operation ID "Label" in="Message" [out="Message"] [error="Error" …]
key ID "Label"                               # correlation key (needs pools)
  prop NAME message="Message" path="expression"
note ID "Text" [on=ID]       group ID "Label" members="A B C"

decision ID "Label" hit=first               # unique | first | any | priority | collect
  | in1: string | in2: number || out: string || annotation |
  | "e-book"    | -           || "manager"   || licences   |
  | -           | <= 150      || "auto"      ||            |

# decision requirements graphs and boxed expressions (language 0.7, header tok 0.7 or tok 1.0)
inputdata ID "Label" [name="FEEL name"] type=number            # also date, "date and time", list, an item …
source ID "Label" [requires="D I S"] [kind="regulation"] [uri="https://…"]
bkm ID "Label" params="a: number, b: string" [type=T] [requires="BKM S"] expr="a * b"
decision ID "Label" [name="FEEL name"] [type=T] requires="D I BKM S" [question="?"] [answers="…"] expr="feel"
decision ID "Label" requires="…" invoke=BKM with="a = feel; b = feel"     # invocation
decision ID "Label" requires="…" hit=rule-order|output-order|collect aggregation=sum|count|min|max default="0"
  | in: number = [0..100] || out: string = "high", "low" |              # input/output values
decision ID "Label"                         # context: entries in order, optional result last
  entry NAME [type=T] expr="feel"           # NAME a word or "quoted name"; each line may have its own block
  entry f params="x: number" expr="x * 2"   # function definition
  entry table                               # nested table or relation
    | col: string | val: number |           # relation: header without ||
    | "a"         | 1           |
  result expr="f(2)"
decision ID "Label"                         # list
  element expr="1"
item T "Label" [type=number] [allowed="[0..10]"] [collection]      # DMN type (0.7)
  item field type=string                    # components
```

FEEL of 0.7 and 1.0 documents is full FEEL (level 3): `for x in l return e`, `some/every x in l satisfies c`, `l[item > 1]`,
`{a: 1}.a`, `@"2027-01-15"`, `date and time(…)`, durations, ranges, names with spaces resolved against the document.

```

form ID "Title"
  text "## Markdown with {{variable}}"
  field KEY "Label" type=textfield|textarea|number|checkbox|select|radio|datetime [required] [options="v:Label, v:Label"]

simulate cases=200 seed=1 arrival="exp(2h)"
resource ID "Label" count=2
variable NAME "unif(20, 300)"               # fixed unif tri norm lnorm exp bern cat; units s m h d
    activity … by=RESOURCE duration="tri(10m, 20m, 1h)" assign="zgoda=bern(0.8)"
    catch … delay="exp(3d)"     flow … p=0.3
```

FEEL in `when`, `expr`, `condition`, `correlation` and decision cells is written **without** a leading `=`.

Compensation and catch-all events (any document):
`service a … compensate=h` + `service h … compensation` · `throw c … compensate [activity=a]` ·
`boundary b on=a error` (any error) · `boundary b on=a escalation` (any escalation)

Executable layer (only with `executable`; values computed at run time start with `=`):

```
user t … assignee="kierownik" candidate-groups="a, b" due="=now() + duration(\"P3D\")" priority=70
user t … candidate-users="ola, jan" follow-up="2027-01-20T09:00:00Z"
user t … parallel collection="items" item=x output=results output-item="x.result" until="numberOfCompletedInstances >= 2"
adhoc a … active="chosen" until="done = true" wait
call c … process=child binding=deployment|latest|version-tag version="v2" child-vars=none parent-vars=none business-id="=numer"
service s … type="job" retries=5 headers="key: value; …" inputs="target = expr; …" outputs="target = expr; …"
service r … connector=rest method=GET url="https://…" result=response [body= query= headers-in=]
end e … message="M" type="job"          catch|boundary … condition="x > 1" events="create, update"
any … listeners="start: job-type; end: job-type (5)"   (user tasks also: creating, assigning, updating, completing, canceling)
tok 0.3 "T" executable version-tag="v1"   pool p … version-tag="v1"
```

Language 0.5 elements (transaction, cancel, complex, loop conditions, multiple events, instantiate, process data,
global tasks, operations, keys) and 0.6 elements (conversations, choreographies, min/max) are BPMN only:
`--level basic|analytic` rejects them, and in executable documents for Camunda they are reported as `C100`–`C123`.
Decisions of 0.7 are level `full`; Camunda codes `C130` (bkm with invoke=), `C131` (non-literal function body),
`C132` (decision without logic).
In 0.5 documents `resource` and `by=` also export BPMN performers.

Commands: `tok check F [--class descriptive|analytic|common-executable|full]` · `tok fmt F [-w]` · `tok bpmn F` ·
`tok svg F` · `tok export F [-o DIR]` (0.7: NAME.dmn + NAME.drd.svg per decision graph) · `tok import F.bpmn|F.dmn [-o F.tok]`
(BPMN: writes F.tok.layout, the drawing reused by bpmn/svg/export; `--no-sidecar`) · `tok sim F` ·
`tok eval F DECISION name=value …` (0.7: with required decisions; JSON values `x='{"a": 1}'`) · `--lang pl` · `--json`

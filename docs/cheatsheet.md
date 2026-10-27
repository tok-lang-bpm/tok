# tok 0.5 — cheat sheet

One statement per line. Blocks by indentation (spaces). `#` starts a comment.
Every statement starts with a keyword. Identifiers: `[A-Za-z_][A-Za-z0-9_-]*`, unique. Labels: "quoted text".
Attributes: `key=value` (value: word, number or "text") and bare flags.

```
tok 0.5 "Title" [executable]                       # header, first line (0.1 and 0.3 documents stay valid)

start ID "Label" [message="M" | timer=P3D | signal="S" | condition="feel"] [form=F]
end ID "Label" [message="M" | error="E" | escalation="X" | signal="S" | terminate]
catch ID "Label" message="M" | timer=PT2H | signal="S" | condition="feel" | link="L"
throw ID "Label" [message="M" | escalation="X" | signal="S" | link="L" | compensate]
boundary ID "Label" on=ACTIVITY timer=P7D [noninterrupting]   # also message/error/escalation/signal
    several triggers = multiple event: message="A" message="B" signal="C"; + parallel = all of them (catching)

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

pool ID "Label" [process=P] [executable]    # with a block: participant + process
  lane ID "Label"                           # lanes nest; nodes go inside lanes
pool ID "Label"                             # no block: black-box pool

flow A -> B -> C                            # chain of sequence flows
flow G -> D "tak" when="amount > 100"       # one flow: label, FEEL condition
flow G -> E "nie" default                   # default flow
message A -> B "Label"                      # message flow between pools (nodes or pools)

data ID "Label" [state="złożony"] [type=ITEM] [collection]
store ID "Label" [type=ITEM]
reads ACTIVITY DATA          writes ACTIVITY DATA     # DATA: data, store, input, output, property
item ID "Label" [structure=Name|xsd:string] [kind=information|physical] [collection]
input ID "Label" [type=ITEM] [collection] [on=ACTIVITY]    output ID … (same)
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
user t … parallel collection="items" item=x output=results output-item="x.result" until="numberOfCompletedInstances >= 2"
adhoc a … active="chosen" until="done = true" wait
call c … process=child binding=deployment|latest|version-tag version="v2" child-vars=none parent-vars=none
service s … type="job" retries=5 headers="key: value; …" inputs="target = expr; …" outputs="target = expr; …"
service r … connector=rest method=GET url="https://…" result=response [body= query= headers-in=]
end e … message="M" type="job"          catch|boundary … condition="x > 1" events="create, update"
any … listeners="start: job-type; end: job-type (5)"   (user tasks also: creating, assigning, updating, completing, canceling)
tok 0.3 "T" executable version-tag="v1"   pool p … version-tag="v1"
```

Language 0.5 elements (transaction, cancel, complex, loop conditions, multiple events, instantiate, process data,
global tasks, operations, keys) are BPMN only: `--level basic|analytic` rejects them, and in executable documents
for Camunda they are reported as `C100`–`C110`. In 0.5 documents `resource` and `by=` also export BPMN performers.

Commands: `tok check F` · `tok fmt F [-w]` · `tok bpmn F` · `tok svg F` · `tok export F [-o DIR]` ·
`tok import F.bpmn` · `tok sim F` · `tok eval F DECISION name=value …` · `--lang pl` · `--json`

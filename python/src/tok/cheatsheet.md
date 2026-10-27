# tok 0.1 — cheat sheet

One statement per line. Blocks by indentation (spaces). `#` starts a comment.
Every statement starts with a keyword. Identifiers: `[A-Za-z_][A-Za-z0-9_-]*`, unique. Labels: "quoted text".
Attributes: `key=value` (value: word, number or "text") and bare flags.

```
tok 0.1 "Title" [executable]                       # header, first line

start ID "Label" [message="M" | timer=P3D | signal="S" | condition="feel"] [form=F]
end ID "Label" [message="M" | error="E" | escalation="X" | signal="S" | terminate]
catch ID "Label" message="M" | timer=PT2H | signal="S" | condition="feel" | link="L"
throw ID "Label" [message="M" | escalation="X" | signal="S" | link="L" | compensate]
boundary ID "Label" on=ACTIVITY timer=P7D [noninterrupting]   # also message/error/escalation/signal

task | user | manual | service | script | rule | send | receive | call   ID "Label"
    flags: loop parallel sequential compensation
    user … form=F        rule … decision=D result=var        script … expr="feel" result=var
    service … type="job"  send/receive … message="M"          call … process=P
sub ID "Label"           # embedded sub-process: indented block with its own start and end
eventsub ID "Label"      # event sub-process: block starts with a start event with a trigger

xor | and | or | eventgw   ID ["Question?"]

pool ID "Label" [process=P] [executable]    # with a block: participant + process
  lane ID "Label"                           # lanes nest; nodes go inside lanes
pool ID "Label"                             # no block: black-box pool

flow A -> B -> C                            # chain of sequence flows
flow G -> D "tak" when="amount > 100"       # one flow: label, FEEL condition
flow G -> E "nie" default                   # default flow
message A -> B "Label"                      # message flow between pools (nodes or pools)

data ID "Label" [state="złożony"] [collection]
store ID "Label"
reads ACTIVITY DATA          writes ACTIVITY DATA
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

Commands: `tok check F` · `tok fmt F [-w]` · `tok bpmn F` · `tok svg F` · `tok export F [-o DIR]` ·
`tok import F.bpmn` · `tok sim F` · `tok eval F DECISION name=value …` · `--lang pl` · `--json`

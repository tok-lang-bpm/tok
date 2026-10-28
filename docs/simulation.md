# Simulation

A tok model with simulation statements is also a discrete-event simulation model. The simulator follows the cases of one process through the diagram, lets them wait for people (resources), draws durations and decisions from distributions, and reports an event log and indicators. Simulation statements never change the BPMN export.

This document describes version 0.1 of the simulator in the reference implementation (`tok.sim`). The statements themselves are defined in the specification, §11.

## What is simulated

The simulator runs **one process**: the process of the first pool that has one, or the top-level process of a document without pools.

| Element | Behaviour |
|---|---|
| start event | each case starts at the plain start event (one without a trigger; if there is none, the first start event) |
| `simulate arrival=` | time between consecutive case arrivals; the first case arrives at `start=` |
| `variable` | drawn once per case, when the case starts |
| activity (`task`, `user`, `service`, `manual`, `script`, `rule`, `send`, `receive`, `call`) | waits for a free performer of `by=` (first come, first served), then takes `duration=` (0 if absent); `assign=` sets variables when it completes |
| `rule` task | evaluates its decision table with the case variables and stores the result in `result=` (default: the single output name). In a document of language 0.7 it evaluates the decision with the decisions, input data and business knowledge models it requires (the case variables are the input data, by FEEL name) and stores the value of the decision; `result=` defaults to the only output of a decision table, otherwise to the FEEL name of the decision |
| `script` task | evaluates its FEEL expression and stores the result in `result=` (in a document of language 0.7 every condition and expression is FEEL of conformance level 3, `feel.md` §9) |
| `xor` split | the first outgoing flow (in document order) whose `when` holds; otherwise one flow drawn by `p=`; otherwise the `default` flow |
| `or` split | every flow whose `when` holds or whose `p=` is drawn (each independently); if none, the `default` flow |
| `and` split | all outgoing flows |
| `and` join | waits for a token on every incoming flow |
| `or` join | waits for as many tokens as the matching `or` split activated |
| `xor` join | passes every token through |
| `eventgw` | races the events after it: a timer fires after its duration, a message or signal after its `delay=`, a receive task after its `delay=` (or `duration=`); the earliest wins |
| `catch` event | waits for its timer or its `delay=`; without either it passes immediately |
| trigger without a value (language 0.5: the flags `message`, `timer`, `signal`, `condition`) | behaves like a message: waits for the event's `delay=` (a `timer` flag has no duration) |
| boundary timer | starts when the activity is reached (including time spent waiting for a performer); if the activity is still running when it fires, an interrupting timer cancels the activity and frees its performer, a non-interrupting one starts an extra token |
| `sub` | the token enters at the sub-process start event; the sub-process completes when all its tokens have finished |
| `end` | the token finishes; `terminate` finishes the whole case (inside a sub-process: the sub-process) |
| `throw` events, message flows, `eventsub` | not simulated in version 0.1 |
| `transaction` (language 0.5) | runs like `sub`. A token at an `end … cancel` inside it (in a nested sub-process: of the nearest transaction) stops every token and activity of the transaction, starts the compensation handlers of the activities completed inside it (latest completed first, all at once, as `throw … compensate` does); when they have finished, the transaction is logged as `interrupted` and the token leaves through the boundary event `on=T cancel`. Without such a boundary event the token ends there: a cancelled transaction never completes and does not take its outgoing flows |
| `complex` split (language 0.5) | like an `or` split (`when`, `p=`, `default`) |
| `complex` join (language 0.5) | without `activation=`, like an `or` join. With it: the number of expected tokens is fixed at the first arrival (the branches the matching `or` or `complex` split activated, otherwise the number of incoming flows); at every arriving token `activation` is evaluated with the case variables and `_activationCount`, the number of tokens arrived so far; the first token for which it holds goes on, later ones are absorbed, and when all expected tokens have arrived the gateway is ready for the next activation. If the condition never holds the case ends in `deadlock`; a condition that cannot be evaluated counts as false and gives a warning |
| standard loop (language 0.5: `loop` with `while=` and/or `loop-max=`) | after iteration k the activity runs again while k < `loop-max` (when given) and `while` holds (when given), evaluated with the case variables after `assign=` and `loopCounter` = k; with `test-before` the same test is made before the first iteration, so the activity may be skipped; each iteration is a row of the log and its boundary events are armed again |
| multiple catch event (language 0.5: several triggers, on `catch`, `boundary` and events after an `eventgw`) | each timer waits its own duration, the other triggers one draw of `delay=` (without `delay=`: at once on a `catch`, never on a boundary event or in a race); the event fires at the earliest of them, with `parallel` at the latest; a multiple start event starts cases like a start event |
| `receive … instantiate`, `eventgw … instantiate` (language 0.5) | when the process has no start event, cases start there: the receive task runs as an activity (its `duration=`; the arrival of the case is the message), the gateway races its events as usual (if none of them has a known wait, the event on its first flow happens at once) |

Times are calendar times counted from `start=` without working-hours calendars: a 6-hour task started at 16:00 ends at 22:00. A case that passes more than 10,000 elements is stopped and reported with the end event `loop-limit`.

Activities with `loop`, `parallel` or `sequential` markers run once in version 0.1; multi-instance collections are simulated from language 0.3 and standard loops with `while=` or `loop-max=` from language 0.5 (the new behaviour applies only to documents that use these elements, so the results of 0.1 and 0.3 documents do not change).

## Distributions

A distribution is a duration (`30m`), a number, or a call. Durations use the units `s`, `m` (minutes), `h` and `d`. Inside `unif`, `tri`, `norm`, `lnorm` and `exp` all arguments are durations or all are numbers.

| Call | Meaning | Example |
|---|---|---|
| `fixed(x)` | always `x` | `fixed(15m)` |
| `unif(a, b)` | uniform between `a` and `b` | `unif(20, 300)` |
| `tri(a, mode, b)` | triangular: minimum, most likely, maximum | `tri(5m, 10m, 20m)` |
| `norm(mean, sd)` | normal; a negative duration becomes 0 | `norm(1h, 10m)` |
| `lnorm(mean, sd)` | log-normal with the given **mean and standard deviation of the result** (not of its logarithm) | `lnorm(6h, 3h)` |
| `exp(mean)` | exponential with the given mean; the usual model of time between arrivals | `exp(4h)` |
| `bern(p)` | `true` with probability `p` | `bern(0.8)` |
| `cat(a=p, b=q, …)` | one of the labels; probabilities sum to 1; labels `true`, `false` and numbers become booleans and numbers | `cat(drukowana=0.7, e-book=0.3)` |

`assign=` lists several variables separated by semicolons: `assign="zgoda=bern(0.8); pilne=bern(0.1)"`.

All draws come from one random number generator seeded with `seed=`. The same document and seed always give the same log. The Rust engine (`tok-sim`, the native `tok sim`) reproduces the generator and the simulator of the reference implementation, so it gives the same log and indicators number by number; the last bit of normal, log-normal and exponential draws comes from the platform's C library, so results can differ in that bit between operating systems.

## Diagnostics

`tok check` reports:

| Code | Meaning |
|---|---|
| `S001` | a distribution (in `arrival`, `variable`, `duration`, `assign`, `delay`) or a probability `p=` is not valid |
| `S002` | the `p=` values of the flows leaving an `xor` gateway do not sum to 1 |
| `S003` | the document has a `simulate` statement, but an `xor` or `or` gateway with several outgoing flows has neither conditions nor probabilities; the simulator takes the first flow |
| `R002` | `by=` names a resource that is not declared |

## Outputs

**Event log** (`tok sim --log log.csv`): one row per activity instance.

| Column | Content |
|---|---|
| `case` | case number, from 1 |
| `activity`, `label` | identifier and label of the activity |
| `resource` | resource of `by=` (empty if none) |
| `enqueue` | when the token reached the activity |
| `start` | when a performer started the work (empty if the activity was interrupted while waiting) |
| `end` | when the activity completed or was interrupted |
| `status` | `completed` or `interrupted` |

The log can be read by process-mining tools: `case` is the case identifier, `label` the activity, `start` and `end` the timestamps. The same log in the XES format, which these tools read without any column mapping, is written by `--xes` (see [XES event log](#xes-event-log)).

**Cases** (`--cases-csv cases.csv`): case number, start, end, end event, cycle time in seconds and the case variables (JSON).

**Indicators** (`tok sim`, `--json`):

| Field | Content |
|---|---|
| `cases.started`, `cases.finished`, `cases.by_end_event` | number of cases, of finished cases and of cases per end event |
| `cycle_time.mean`, `median`, `p90` | cycle time from arrival to end (mean, median, 90th percentile); `…_s` in seconds |
| `activities.ID.count`, `mean_wait`, `mean_processing` | completed instances, mean time waiting for a performer, mean processing time |
| `resources.ID.utilization` | busy time divided by `count` × simulated period |
| `horizon_s` | simulated period: from the first arrival to the last event |
| `warnings` | conditions or expressions the simulator could not evaluate |

## XES event log

`tok sim FILE --xes log.xes` writes the event log as an XES document (IEEE 1849-2016, eXtensible Event Stream), the exchange format of process-mining tools such as ProM, pm4py, bupaR, Disco and Celonis. It can be combined with `--log`, `--cases-csv` and `--json`. The same document comes from `SimResult.to_xes()` and `SimResult.write_xes(path)` of the Python package and from `tok_xes()` of the R package. XES export is a feature of the Rust engine: the pure-Python implementation, frozen at language 0.3, does not have it.

**Structure.** The document follows the XML schema of IEEE 1849-2016: the elements have no XML namespace, and the log element has `xes.version="1.0"` (the schema types the attribute as a decimal; `1.0` is the value OpenXES and ProM write, so every reader accepts it). In schema order the log contains:

1. the standard extensions Concept (`concept`), Time (`time`), Lifecycle (`lifecycle`) and Organizational (`org`), with their definition URIs at `http://www.xes-standard.org/`;
2. global attributes: every trace has `concept:name` and `start`; every event has `concept:name`, `concept:instance`, `lifecycle:transition`, `time:timestamp` and `activity`;
3. the classifiers `Activity` (`concept:name`), `Activity_lifecycle` (`concept:name lifecycle:transition`) and `Activity_id` (`activity`);
4. the log attributes `lifecycle:model` = `standard` and `seed`;
5. one trace per case, in case order.

**Traces.** A trace has the attributes:

| Key | Type | Content |
|---|---|---|
| `concept:name` | string | case number, from 1 |
| `start` | date | arrival of the case |
| `end` | date | end of the case (absent for an unfinished case) |
| `end_event` | string | identifier of the end event (absent for an unfinished case) |
| `cycle_time_s` | float | cycle time in seconds, as in the table of cases |
| one key per case variable | see below | the value at the end of the case |

Case variables are typed: booleans are `boolean`; integers are `int` (as text when they do not fit in 64 bits); other numbers are `float`; durations are `float` seconds; text that is exactly an ISO date or date-time (as FEEL dates and date-times are stored, for example `2027-03-01`) is `date`; any other text is `string`; lists and contexts are `string` holding compact JSON. A variable without a value has no attribute. Keys are XML names: characters other than letters, digits, `_`, `-` and `.` become `_` (`:` too, because XES reserves it for extension prefixes), and a name that collides with one of the keys above gets the prefix `variable_`.

**Events.** Each row of the event log, one activity instance, gives up to three events in the transitions of the standard lifecycle model:

| Transition | Time | When |
|---|---|---|
| `schedule` | `enqueue` | always: the case reached the activity |
| `start` | `start` | a performer started the work |
| `complete` | `end` | the activity completed (`status` `completed`) |
| `ati_abort` | `end` | the activity was interrupted by a boundary timer while it was running |
| `withdraw` | `end` | the activity was interrupted by a boundary timer while it was waiting for a performer (no `start`) |

An interrupting boundary timer cancels only the activity, not the case, so the transition is `ati_abort` (activity instance aborted), not `pi_abort` (process instance aborted); the simulator never aborts a whole case on its own, so `pi_abort` does not occur. Every event has `concept:name` (the label of the activity, or its identifier when it has no label), `concept:instance` (the row number of the instance in the event log, from 1, shared by its events), `lifecycle:transition`, `time:timestamp`, `org:resource` (the resource of `by=`, only when the activity has one) and `activity` (the identifier of the activity). Within a trace the events are ordered by time; events at the same time keep the order of the event log and of the lifecycle.

**Timestamps** are `xs:dateTime` values with milliseconds, for example `2027-03-01T09:49:56.984+01:00`. The simulator counts time from `start=` of `simulate`, and the timestamps keep its UTC offset: `start="2027-03-01T09:00+01:00"` gives times with `+01:00`; a `start=` without an offset (as the default `2027-01-04T08:00`) gives times without one, which process-mining tools read as local time. An offset that is not a whole number of minutes or exceeds 14 hours (outside `xs:dateTime`) is converted to UTC. Fractions of a millisecond are cut off, as Python's `isoformat(timespec="milliseconds")` does.

The output is deterministic: the same document and seed give the same bytes, encoded in UTF-8 with `\n` line endings; attribute values are escaped for XML, and characters XML 1.0 does not allow become U+FFFD.

## Example

`examples/kwerenda-symulacja.tok` models genealogical queries in a state archive. A registry clerk registers the request; one of two archivists assesses it; 15% of requests are refused, the rest are searched for and answered.

```
simulate cases=300 seed=7 arrival="exp(4h)"
resource archiwisci "Archiwiści" count=2
resource kancelisci "Kanceliści" count=1
```

```
tok sim examples/kwerenda-symulacja.tok
```

Indicators of the reference implementation (seed 7):

| Indicator | Value |
|---|---|
| cases by end event | 252 answered, 48 refused |
| cycle time | mean 17.5 h, median 14.3 h, 90th percentile 1.5 d |
| archivists' utilization | 81% |
| clerk's utilization | 9% |
| assessment of the request | waits 4.2 h, takes 31 min |
| search in the holdings | waits 4.0 h, takes 5.8 h |
| preparation of the answer | waits 3.8 h, takes 1.1 h |
| registration, sending | wait under 1 minute |

**Reading the result.** One request arrives every 4 hours on average, and each needs about 0.5 h + 0.85 × (6 h + 1.2 h) ≈ 6.6 h of an archivist's time, so two archivists are busy about 6.6 / (2 × 4) ≈ 82% of the time. The simulation confirms it (81%). At this load the queues before the archivists' three tasks add about 12 hours to every answered request, roughly two thirds of the mean cycle time. The registry, at 9% utilization, is not a bottleneck. Two changes are worth testing by editing the document and running it again: a third archivist (`count=3`), or a shorter assessment that refuses impossible requests earlier, before they wait for the search.

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
| `rule` task | evaluates its decision table with the case variables and stores the result in `result=` (default: the single output name) |
| `script` task | evaluates its FEEL expression and stores the result in `result=` |
| `xor` split | the first outgoing flow (in document order) whose `when` holds; otherwise one flow drawn by `p=`; otherwise the `default` flow |
| `or` split | every flow whose `when` holds or whose `p=` is drawn (each independently); if none, the `default` flow |
| `and` split | all outgoing flows |
| `and` join | waits for a token on every incoming flow |
| `or` join | waits for as many tokens as the matching `or` split activated |
| `xor` join | passes every token through |
| `eventgw` | races the events after it: a timer fires after its duration, a message or signal after its `delay=`, a receive task after its `delay=` (or `duration=`); the earliest wins |
| `catch` event | waits for its timer or its `delay=`; without either it passes immediately |
| boundary timer | starts when the activity is reached (including time spent waiting for a performer); if the activity is still running when it fires, an interrupting timer cancels the activity and frees its performer, a non-interrupting one starts an extra token |
| `sub` | the token enters at the sub-process start event; the sub-process completes when all its tokens have finished |
| `end` | the token finishes; `terminate` finishes the whole case (inside a sub-process: the sub-process) |
| `throw` events, message flows, `eventsub` | not simulated in version 0.1 |

Times are calendar times counted from `start=` without working-hours calendars: a 6-hour task started at 16:00 ends at 22:00. A case that passes more than 10,000 elements is stopped and reported with the end event `loop-limit`.

Activities with `loop`, `parallel` or `sequential` markers run once in version 0.1.

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

The log can be read by process-mining tools: `case` is the case identifier, `label` the activity, `start` and `end` the timestamps.

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

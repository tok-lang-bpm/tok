# Calibration of simulation parameters

The simulation statements of a tok document (spec §11) say how long activities take, how often cases arrive and which branches they take. The **fit layer** estimates these parameters from an **event log** — the log of a real process exported from a workflow system or a process-mining tool, or the log of a simulation — and writes them back into the document, so that the model simulates the process that produced the log. It is part of the Julia package (`Tok.Fit`, [julia.md](julia.md)) and changes nothing in the language: a fitted document is an ordinary document of the same language version.

## What is fitted

| Parameter | Statement | Observations in the log |
|---|---|---|
| processing time of an activity | `duration=` of a `task`, `user`, `service`, `manual`, `script`, `rule`, `send`, `receive` or `call` | `end − start` of every completed instance of the activity; an instance interrupted by a boundary event after it started is a right-censored observation (the work would have taken longer than `end − start`) |
| time between arrivals | `arrival=` of `simulate` | the differences between consecutive arrivals of cases |
| branch probabilities | `p=` of the flows leaving an `xor` or `or` gateway | the number of times each branch was taken, and the number of times the gateway was reached |

Each duration is fitted in the **family** the model declares for it (`exp`, `norm`, `lnorm`, `unif`, `tri` or a fixed value; `lnorm` for an activity without `duration=`), or in the family given in the option `families` (`Dict("ocena" => :exp, "arrival" => :exp)`). Instances of zero length are left out of the `exp` and `lnorm` fits (these families have no mass at zero), and censored instances are not used by the `unif`, `tri` and fixed fits; both are reported in the notes. Waiting times are not parameters: they follow from the arrivals, the durations and the number of performers, which the simulation reproduces.

**Branches.** A log records activities, not sequence flows, so the branch a case took at a gateway is recognized by what follows it. For each flow with `p=`, the fit follows the flow through elements that are not logged (events and gateways) to the first activities on it; if these activities can be reached only through that flow of that gateway, every instance of them counts as one passage through the branch. A branch that reaches no activity but end events of its own is counted by the end events of the cases, when the log has them (the XES log of `tok sim`, or the table of cases). The number of passages through the gateway is the sum over its branches, or, for an inclusive gateway and for an exclusive gateway with one branch that cannot be counted, the number of completions of the activities (and starts) from which a single path leads to the gateway. A gateway whose branches cannot be told apart in this way, an inclusive gateway without a default flow, and a gateway with conditions (`when=`) are reported in the notes and keep their probabilities.

Not fitted: the number of performers of a resource (`count=`), the distributions of case variables (`variable`, `assign=`) and the waiting times of events (`delay=`, timers), which the event log does not record.

## Event logs

| Reader | Format |
|---|---|
| `read_xes(path or text)` | XES (IEEE 1849-2016), as `tok sim --xes` writes it (see [simulation.md](simulation.md#xes-event-log)) and as process-mining tools export it |
| `read_csv_log(log; cases=nothing)` | the CSV event log of `tok sim --log` (columns `case`, `activity`, `label`, `resource`, `enqueue`, `start`, `end`, `status`), optionally with the table of cases of `--cases-csv` |

In an XES log the activity instances are formed from the lifecycle transitions of the events: `schedule` gives the time the case reached the activity, `start` the start of the work, `complete` the end of a completed instance, `ati_abort` and `withdraw` the end of an interrupted one. Events are grouped into instances by `concept:instance`; without it, by activity in the order of the events. The activity is the attribute `activity` (the identifier of the element in the document), or else `concept:name`, which then has to be an identifier or a label of the document. The trace attributes `start` and `end_event` give the arrival and the end event of a case; without `start` the arrival is the first enqueue time of the case.

Times are ISO 8601 date-times; a time with a UTC offset is converted to UTC. The CSV log of `tok sim` has times to the second, its XES log to the millisecond.

## Maximum-likelihood estimates

`fit(model, log; level=0.95, families=Dict(), bootstrap=200, seed=1)` gives the maximum-likelihood estimate of every parameter, with an interval at the confidence `level`:

| Family | Estimate | Interval |
|---|---|---|
| `exp(mean)` | total observed time (censored instances included) divided by the number of completed instances | Wald interval on the log scale |
| `norm(mean, sd)` | sample mean and standard deviation (with censoring: numerical maximization of the likelihood with the survival function) | Wald intervals from the observed information (the standard deviation on the log scale) |
| `lnorm(mean, sd)` | the normal fit of the logarithms, transformed to the mean and standard deviation of the duration, which are the parameters of the language | delta method on the log scale |
| `unif(a, b)` | smallest and largest observation | parametric bootstrap (percentiles of `bootstrap` refits of samples drawn from the estimate) |
| `tri(a, mode, b)` | numerical maximization: for given bounds the most likely mode is one of the observations, the bounds are found by Nelder–Mead | parametric bootstrap |
| `p=` | the share of passages through the gateway that took the branch | Wilson score interval |

The bootstrap draws from a random stream given by `seed`, so the intervals are reproducible. For `unif` and `tri` the support depends on the parameters and the usual asymptotic theory does not hold; the bootstrap intervals are then the honest ones.

## Bayesian estimates

`fit_bayes(model, log; samples=1000, warmup=500, seed=1, level=0.95, families=Dict())` samples the posterior of every parameter with the No-U-Turn sampler of Turing.jl. It is available after `using Turing` (the package extension `TokTuringExt`; Turing is not a dependency of Tok.jl). Each parameter has its own model, with the same likelihood as the maximum-likelihood fit (including censored instances) and weakly informative priors on the scale of the data (`m` is the median of the observations):

| Family | Priors |
|---|---|
| `exp` | log of the mean ~ Normal(log m, 3) |
| `lnorm` | μ ~ Normal(log m, 3), σ ~ half-Normal(2) (parameters of the logarithm) |
| `norm` | mean ~ Normal(m, 10 m), sd ~ half-Normal(10 m) |
| `unif`, `tri` | bounds beyond the observed range by span·e^u with u ~ Normal(−3, 2); the mode of `tri` at a share logistic(v) of the range, v ~ Normal(0, 1.5) |
| `p=` of an exclusive gateway | the shares of its branches (and of the default flow) ~ Dirichlet(1, …, 1), counts multinomial |
| `p=` of an inclusive gateway | each p ~ Beta(1, 1), counts binomial |

The estimates are posterior means of the parameters of the language (for `lnorm`, of the mean and standard deviation of the duration, computed for every draw); the intervals are equal-tailed credible intervals at `level`. With logs of a few hundred cases the posterior means differ from the maximum-likelihood estimates by a fraction of their uncertainty.

## Results

`fit` and `fit_bayes` return a `FitResult`: its `estimates`, the `level` of their intervals and `notes` on what could not be estimated. `table(f)` lists one row per parameter:

| Column | Content |
|---|---|
| `element` | the activity, `arrival`, or the flow `GATEWAY -> TARGET` |
| `parameter` | `:duration`, `:arrival` or `:p` |
| `family` | `:exp`, `:norm`, `:lnorm`, `:unif`, `:tri`, `:fixed`, or `:p` |
| `name` | the argument of the language: `mean`, `sd`, `a`, `mode`, `b`, `x` or `p` |
| `estimate`, `lower`, `upper` | the estimate and its interval; durations in seconds |
| `n`, `censored` | completed and censored observations (for a probability, the passages through the gateway) |
| `method` | `:mle` or `:bayes` |

`apply(text, f; digits=4)` writes the estimates into the document — `duration=` of the activities, `arrival=` of `simulate` (added when the document has none) and `p=` of the flows — rounded to `digits` significant digits, with durations in the largest unit that keeps the number at least 1 (`lnorm(30.01m, 10.33m)`). The document is printed in canonical form by the independent core of Tok.jl; comments are kept.

## Example

```julia
using Tok
const F = Tok.Fit

text = read("examples/kwerenda-symulacja.tok", String)
log = F.read_xes(Tok.Engine.sim_xes(text; cases=3000, seed=11))
f = F.fit(text, log)
for r in F.table(f)
    println(rpad(r.element, 26), rpad("$(r.family) $(r.name)", 14), round(r.estimate; sigdigits=4),
            "  [", round(r.lower; sigdigits=4), ", ", round(r.upper; sigdigits=4), "]")
end
```

With 3000 simulated cases of the example (true values: assessment `lnorm(30m, 10m)`, search `lnorm(6h, 3h)`, arrivals `exp(4h)`, 85 % of requests possible) the fit gives:

| Parameter | Estimate | 95 % interval |
|---|---|---|
| `ocena` lnorm mean | 30.0 min | 29.6 – 30.4 min |
| `ocena` lnorm sd | 10.3 min | 10.0 – 10.7 min |
| `poszukiwania` lnorm mean | 5.95 h | 5.84 – 6.07 h |
| `poszukiwania` lnorm sd | 3.01 h | 2.89 – 3.14 h |
| `arrival` exp mean | 3.97 h | 3.83 – 4.12 h |
| `mozliwa -> poszukiwania` p | 0.851 | 0.837 – 0.863 |

The same calibration from the command line of the Julia application: `tok fit examples/kwerenda-symulacja.tok log.xes -o fitted.tok`.

## Tests

The tests of Tok.jl simulate models with known parameters in the engine, fit the log and require the estimates to recover the parameters: within 5 % (10 % for standard deviations) and inside the 99.9 % interval for the example above, for durations censored by boundary timers (`exp` and `lnorm`), for the branches of an inclusive gateway, and for branches told apart only by their end events; the CSV and XES logs of the same run give the same estimates. The Bayesian fit is tested in the same way in an environment with Turing ([julia.md](julia.md#tests)).

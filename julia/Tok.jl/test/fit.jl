# Calibration (Tok.Fit): event logs, and the round trip simulate with known parameters ->
# fit -> recover the parameters.

const F = Tok.Fit

"The estimate of one parameter in a fit result."
function estimate(f, element, name)
    for e in f.estimates
        e.element == element || continue
        k = findfirst(==(name), e.names)
        k === nothing || return (value=e.values[k], lower=e.lower[k], upper=e.upper[k], family=e.family,
                                 censored=e.censored)
    end
    error("no estimate of $name for $element")
end

"Within `rel` of the true value and inside the interval (the fits below use 99.9 % intervals)."
recovered(e, truth; rel=0.05) = abs(e.value - truth) <= rel * abs(truth) && e.lower <= truth <= e.upper

@testset "event logs" begin
    @test F.parse_time("1970-01-02") == 86400
    @test F.parse_time("2027-03-01T09:49:56.984+01:00") ≈ F.parse_time("2027-03-01T08:49:56.984") atol = 1e-6
    @test F.parse_time("2027-01-04T08:00:00") - F.parse_time("2027-01-04T08:00") == 0
    r = E.simulate(KWERENDA; cases=50, seed=2, xes=true, csv=true)
    lx = F.read_xes(r.xes)
    lc = F.read_csv_log(r.log_csv; cases=r.cases_csv)
    l0 = F.read_csv_log(r.log_csv)
    for l in (lx, lc, l0)
        @test length(l.traces) == 50
        @test sum(length(t.instances) for t in l.traces) == length(r.log)
    end
    @test lx.has_end_events && lc.has_end_events && !l0.has_end_events
    t = lx.traces[1]
    @test t.case == "1" && t.end_event in ("zakonczona", "odrzucony")
    @test t.instances[1].activity == "rejestracja" && t.instances[1].resource == "kancelisci"
    @test t.instances[1].completed
    # the XES times have milliseconds, the CSV times seconds
    @test abs(lx.traces[3].arrival - lc.traces[3].arrival) < 1
    @test F.csv_rows("a,\"b,\"\"c\"\"\"\r\n1,2\n") == [["a", "b,\"c\""], ["1", "2"]]
end

@testset "calibration: round trip" begin
    r = E.simulate(KWERENDA; cases=3000, seed=11, log=false, cases_table=false, xes=true, csv=true)
    f = F.fit(KWERENDA, F.read_xes(r.xes); bootstrap=40, level=0.999)
    @test isempty(f.notes)
    @test recovered(estimate(f, "ocena", "mean"), 1800)
    @test recovered(estimate(f, "ocena", "sd"), 600; rel=0.1)
    @test recovered(estimate(f, "poszukiwania", "mean"), 21600)
    @test recovered(estimate(f, "poszukiwania", "sd"), 10800; rel=0.1)
    @test recovered(estimate(f, "arrival", "mean"), 14400)
    @test estimate(f, "arrival", "mean").family === :exp
    @test recovered(estimate(f, "mozliwa -> poszukiwania", "p"), 0.85; rel=0.02)
    @test recovered(estimate(f, "mozliwa -> odmowa", "p"), 0.15; rel=0.12)
    for (id, a, c, b) in (("rejestracja", 300, 600, 1200), ("opracowanie", 1800, 3600, 7200))
        @test estimate(f, id, "a").family === :tri
        @test abs(estimate(f, id, "a").value - a) < 0.03 * (b - a)
        @test abs(estimate(f, id, "mode").value - c) < 0.05 * (b - a)
        @test abs(estimate(f, id, "b").value - b) < 0.03 * (b - a)
    end
    # the CSV files give the same estimates up to their resolution of one second
    g = F.fit(KWERENDA, F.read_csv_log(r.log_csv; cases=r.cases_csv); bootstrap=0)
    @test abs(estimate(g, "ocena", "mean").value - estimate(f, "ocena", "mean").value) < 1
    @test estimate(g, "mozliwa -> odmowa", "p").value == estimate(f, "mozliwa -> odmowa", "p").value
    # another family on request
    h = F.fit(KWERENDA, F.read_xes(r.xes); families=Dict("ocena" => :exp), bootstrap=0, level=0.999)
    @test estimate(h, "ocena", "mean").family === :exp
    @test recovered(estimate(h, "ocena", "mean"), 1800)
    # the fitted document is valid and canonical, and simulates like the original
    fitted = F.apply(KWERENDA, f)
    @test isempty(filter(d -> d["severity"] == "error", E.check(fitted)))
    @test E.fmt(fitted) == fitted
    @test occursin("p=0.8", fitted) && occursin("arrival=\"exp(", fitted)
    rows = F.table(f)
    @test length(rows) == sum(length(e.names) for e in f.estimates)
    @test all(r -> r.lower <= r.estimate <= r.upper, rows)
end

const CENSORED = """
tok 0.3 "Terminy"

start s "Wniosek"
task a "Rozpatrzenie" duration="exp(2h)"
boundary t "Termin" on=a timer=PT3H
task b "Odpowiedź" duration="lnorm(1h, 30m)"
boundary t2 "Drugi termin" on=b timer=PT1H30M
end e "Załatwiony"
end late "Po terminie"
end late2 "Po drugim terminie"

flow s -> a -> b -> e
flow t -> late
flow t2 -> late2

simulate cases=4000 seed=3 arrival="exp(1h)"
"""

@testset "calibration: interrupted instances are censored" begin
    r = E.simulate(CENSORED; log=false, cases_table=false, xes=true)
    f = F.fit(CENSORED, F.read_xes(r.xes); level=0.999)
    a = estimate(f, "a", "mean")
    @test a.censored > 500
    @test recovered(a, 7200)
    @test recovered(estimate(f, "b", "mean"), 3600)
    @test recovered(estimate(f, "b", "sd"), 1800; rel=0.1)
    # leaving the interrupted instances out would underestimate the mean
    naive = [i.finish - i.start for t in F.read_xes(r.xes).traces for i in t.instances if i.activity == "a" && i.completed]
    @test sum(naive) / length(naive) < 0.8 * 7200
end

const INCLUSIVE = """
tok 0.3 "Zakres"

start s "Wniosek"
task r "Rejestracja" duration="10m"
or o "Które części?"
task x "Część X" duration="1h"
task y "Część Y" duration="2h"
task z "Bez części" duration="5m"
or j
end e "Koniec"

flow s -> r -> o
flow o -> x p=0.6
flow o -> y p=0.3
flow o -> z default
flow x -> j
flow y -> j
flow z -> j
flow j -> e

simulate cases=3000 seed=2 arrival="exp(1h)"
"""

const TWO_ENDS = """
tok 0.1 "Dwa zakończenia"

start s
task a "Praca" duration="exp(1h)"
xor g
end e1 "Pierwsze"
end e2 "Drugie"

flow s -> a -> g
flow g -> e1 p=0.3
flow g -> e2 p=0.7

simulate cases=2000 seed=4
"""

@testset "calibration: branches" begin
    r = E.simulate(INCLUSIVE; log=false, cases_table=false, xes=true)
    f = F.fit(INCLUSIVE, F.read_xes(r.xes); level=0.999)
    @test recovered(estimate(f, "o -> x", "p"), 0.6; rel=0.05)
    @test recovered(estimate(f, "o -> y", "p"), 0.3; rel=0.1)
    # branches without activities are told apart by the end events of the cases
    r = E.simulate(TWO_ENDS; xes=true, csv=true)
    f = F.fit(TWO_ENDS, F.read_xes(r.xes); level=0.999)
    @test recovered(estimate(f, "g -> e1", "p"), 0.3; rel=0.1)
    g = F.fit(TWO_ENDS, F.read_csv_log(r.log_csv))
    @test !any(e -> e.parameter === :p, g.estimates)
    @test any(n -> occursin("cannot be told apart", n), g.notes)
end

@testset "calibration: Bayesian fit needs Turing" begin
    if Base.get_extension(Tok, :TokTuringExt) === nothing
        @test_throws ErrorException F.fit_bayes(KWERENDA, F.EventLog(F.Trace[], false))
    else
        @test true
    end
end

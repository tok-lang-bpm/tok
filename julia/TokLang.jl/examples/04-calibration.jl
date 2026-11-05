# Calibration: estimates the parameters of a simulation model from an event log. The log
# comes from the engine here; a real log is read the same way with `read_xes` or
# `read_csv_log`.
using TokLang

const E = TokLang.Engine
const F = TokLang.Fit

text = read(joinpath(@__DIR__, "kwerenda-symulacja.tok"), String)

log = mktempdir() do dir
    path = joinpath(dir, "log.xes")
    write(path, E.sim_xes(text; cases=400, seed=2))
    F.read_xes(path)
end

fit = F.fit(text, log; bootstrap=50)
for row in F.table(fit)
    println(rpad(row.element, 28), rpad(row.name, 6), round(row.estimate; sigdigits=4))
end

fitted = F.apply(text, fit)
println("Errors in the fitted document: ", count(d -> d["severity"] == "error", E.check(fitted)))

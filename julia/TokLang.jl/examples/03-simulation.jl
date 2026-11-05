# The native simulator: one run, then scenarios compared over many replications.
# Start Julia with several threads (`julia -t auto`) to run the replications in parallel.
using TokLang

const S = TokLang.Sim

text = read(joinpath(@__DIR__, "kwerenda-symulacja.tok"), String)
model = S.model(text)

run = S.simulate(model; seed=7)
println("Cycle time of one run (seconds): ", S.cycle_time(run))

# common random numbers make the differences between the scenarios more precise
three = S.scenario(model; name="three archivists", resources=Dict("archiwisci" => 3))
faster = S.scenario(model; name="shorter search", durations=Dict("poszukiwania" => "lnorm(5h, 2h)"))
experiment = S.experiment([model, three, faster]; replications=20, seed=1)

for row in S.summarize(experiment)
    row.indicator == "cycle_time_mean_s" || continue
    println(rpad(row.scenario, 18), round(row.mean / 3600; digits=1), " h ± ", round(row.half_width / 3600; digits=1))
end

# Workload run while the application is compiled: the commands of the application on the
# examples of the repository, so that they start without compiling.
using TokLang

const EXAMPLES = joinpath(@__DIR__, "..", "TokLang.jl", "examples")
path = joinpath(EXAMPLES, "kwerenda-symulacja.tok")
io = IOBuffer()
TokLang.app_main(["core", "fmt", path]; out=io)
TokLang.app_main(["core", "check", path]; out=io)
if TokLang.Engine.available()
    TokLang.app_main(["experiment", path, "--replications", "2", "--cases", "20", "--resource", "archiwisci=3"]; out=io)
    mktempdir() do dir
        xes = joinpath(dir, "log.xes")
        write(xes, TokLang.Engine.sim_xes(read(path, String); cases=100))
        TokLang.app_main(["fit", path, xes]; out=io)
        TokLang.app_main(["fit", path, xes, "--json"]; out=io)
    end
    TokLang.Engine.check(read(path, String))
    TokLang.Engine.fmt(read(path, String))
end

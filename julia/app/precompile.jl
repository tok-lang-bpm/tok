# Workload run while the application is compiled: the commands of the application on the
# examples of the repository, so that they start without compiling.
using Tok

const EXAMPLES = joinpath(@__DIR__, "..", "..", "examples")
path = joinpath(EXAMPLES, "kwerenda-symulacja.tok")
io = IOBuffer()
Tok.app_main(["core", "fmt", path]; out=io)
Tok.app_main(["core", "check", path]; out=io)
if Tok.Engine.available()
    Tok.app_main(["experiment", path, "--replications", "2", "--cases", "20", "--resource", "archiwisci=3"]; out=io)
    mktempdir() do dir
        xes = joinpath(dir, "log.xes")
        write(xes, Tok.Engine.sim_xes(read(path, String); cases=100))
        Tok.app_main(["fit", path, xes]; out=io)
        Tok.app_main(["fit", path, xes, "--json"]; out=io)
    end
    Tok.Engine.check(read(path, String))
    Tok.Engine.fmt(read(path, String))
end

# The `tok` application of TokLang.jl (TokLang.app_main), as julia/app compiles it.

@testset "application" begin
    path = joinpath(ROOT, "examples", "kwerenda-symulacja.tok")
    io = IOBuffer()
    @test TokLang.app_main(["experiment", path, "--replications", "4", "--cases", "50", "--resource", "archiwisci=3"];
                       out=io) == 0
    text = String(take!(io))
    @test occursin("2 scenario(s) × 4 replications", text) && occursin("archiwisci=3", text)
    @test occursin("utilization:archiwisci", text)
    mktempdir() do dir
        xes = joinpath(dir, "log.xes")
        write(xes, E.sim_xes(KWERENDA; cases=400, seed=2))
        out = joinpath(dir, "fitted.tok")
        @test TokLang.app_main(["fit", path, xes, "-o", out]; out=io) == 0
        text = String(take!(io))
        @test occursin("lnorm mean", text) && occursin("mozliwa -> odmowa", text)
        @test isfile(out) && startswith(read(out, String), "tok 0.1")
        @test TokLang.app_main(["fit", path, xes, "--json"]; out=io) == 0
        @test TokLang.JSON.parse_json(String(take!(io)))[1]["method"] == "mle"
    end
    @test TokLang.app_main(["core", "fmt", path]; out=io) == 0
    @test String(take!(io)) == fmt(parse_tok(read(path, String))[1])
    @test TokLang.app_main(["--version"]) == 0
    @test TokLang.app_main(["experiment"]; out=io) == 2
    @test TokLang.app_main(String[]; out=io) == 2
end

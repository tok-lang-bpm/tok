using Aqua
using Test
using TokLang

# The models and the corpus are copies inside the package (tools/julia-data.sh of the
# repository), so that the tests also run from an installed package.
const EXAMPLES = normpath(joinpath(@__DIR__, "..", "examples"))
const CONFORMANCE = joinpath(@__DIR__, "data", "conformance")

readtext(path) = replace(read(path, String), "\r\n" => "\n")

# Runs a script of examples/ in a module of its own, as `julia examples/NAME` would.
function run_example(name)
    redirect_stdout(devnull) do
        Base.include(Module(), joinpath(EXAMPLES, name))
    end
    @test true
end

@testset "quality of the package (Aqua)" begin
    Aqua.test_all(TokLang)
end

@testset "conformance (core)" begin
    cases = sort(filter(f -> endswith(f, ".tok"), readdir(joinpath(CONFORMANCE, "cases"))))
    @test length(cases) >= 12
    for name in cases
        stem = name[1:end-4]
        text = read(joinpath(CONFORMANCE, "cases", name), String)
        doc, diags = parse_tok(text)
        expected = joinpath(CONFORMANCE, "expected", stem)
        @testset "$stem" begin
            @test fmt(doc) == readtext(expected * ".tok")
            codes = sort([d.code for d in diags if d.code in TokLang.CORE_CODES])
            @test join(codes .* "\n") == readtext(expected * ".diag")
            has_errors = any(d -> severity(d) == "error", diags)
            @test isfile(expected * ".bpmn") == !has_errors
            if !has_errors
                @test semantic_xml(doc) == readtext(expected * ".bpmn")
            end
        end
    end
end

@testset "fmt is idempotent" begin
    for name in filter(f -> endswith(f, ".tok"), readdir(joinpath(CONFORMANCE, "expected")))
        text = readtext(joinpath(CONFORMANCE, "expected", name))
        doc, diags = parse_tok(text)
        if !any(d -> severity(d) == "error", diags)
            @test fmt(doc) == text
        end
    end
end

@testset "messages" begin
    doc, diags = parse_tok("tok 0.1\nprocez x\n")
    @test length(diags) == 1
    @test message(diags[1]) == "unknown keyword 'procez'"
    @test message(diags[1], "pl") == "nieznane słowo kluczowe „procez”"
    @test format_diag(diags[1], "pl") == "wiersz 2: błąd P001: nieznane słowo kluczowe „procez”"
end

@testset "example of the core" begin
    run_example("01-core.jl")
end

# The Rust engine, the native simulator and the calibration need the engine library, which
# tok_jll has for every platform of its recipe.
if TokLang.Engine.available()
    include("engine.jl")
    include("sim.jl")
    include("fit.jl")
    include("app.jl")
    @testset "examples of the engine" begin
        foreach(run_example, ("02-engine.jl", "03-simulation.jl", "04-calibration.jl"))
    end
else
    @info "tok engine library not available: the tests of TokLang.Engine, TokLang.Sim and TokLang.Fit are skipped"
end

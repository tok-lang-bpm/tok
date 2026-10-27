using Test
using Tok

const CONFORMANCE = normpath(joinpath(@__DIR__, "..", "..", "..", "conformance"))

readtext(path) = replace(read(path, String), "\r\n" => "\n")

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
            codes = sort([d.code for d in diags if d.code in Tok.CORE_CODES])
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
    @test format_diag(diags[1], "pl") == "linia 2: błąd P001: nieznane słowo kluczowe „procez”"
end

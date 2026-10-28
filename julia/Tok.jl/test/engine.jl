# The Rust engine through the C interface (Tok.Engine).

const E = Tok.Engine

@testset "engine: library" begin
    @test E.version() == Tok.VERSION_STRING
    @test E.abi_version() == 1
    @test isfile(E.library())
end

@testset "engine: check and fmt agree with the corpus and the core" begin
    for name in sort(filter(f -> endswith(f, ".tok"), readdir(joinpath(CONFORMANCE, "cases"))))
        stem = name[1:end-4]
        text = read(joinpath(CONFORMANCE, "cases", name), String)
        diags = E.check(text)
        codes = Set(d["code"] for d in diags)
        expected = split(readtext(joinpath(CONFORMANCE, "expected", stem * ".diag")))
        @test all(c -> c in codes, expected)
        doc, core_diags = parse_tok(text)
        printed = try
            E.fmt(text)
        catch e
            e isa E.TokError && e.kind == "document" || rethrow()
            nothing
        end
        if printed === nothing
            # the engine prints only documents without syntax errors
            @test any(d -> severity(d) == "error", core_diags)
        else
            # the engine and the independent core print the same canonical text
            @test printed == fmt(doc) == readtext(joinpath(CONFORMANCE, "expected", stem * ".tok"))
        end
    end
    d = E.check("tok 0.1\nprocez x\n"; lang="pl")
    @test length(d) == 1 && d[1]["code"] == "P001" && d[1]["message"] == "nieznane słowo kluczowe „procez”"
    @test E.bpmn_class(read(joinpath(ROOT, "examples", "wypozyczenie.tok"), String)) in
          ("descriptive", "analytic", "common-executable", "full")
    text = read(joinpath(CONFORMANCE, "cases", "47-conversation.tok"), String)
    @test any(d -> d["code"] == "R075", E.check(text; class="descriptive"))
end

@testset "engine: export and import" begin
    text = read(joinpath(ROOT, "examples", "propozycja-zakupu.tok"), String)
    @test occursin("bpmndi:BPMNDiagram", E.bpmn(text))
    @test !occursin("bpmndi:BPMNDiagram", E.bpmn(text; layout=false))
    @test occursin("<svg", E.svg(text; title="Propozycja"))
    files = E.export_files(text; name="p")
    @test haskey(files, "p.bpmn") && haskey(files, "p.svg")
    @test any(endswith(".dmn"), keys(files)) && any(endswith(".form"), keys(files))
    xml = read(joinpath(CONFORMANCE, "import", "01-wypozyczenie-pierwszy-model.bpmn"), String)
    t, w = E.import_bpmn(xml)
    @test startswith(t, "tok ") && w isa Vector{String}
    r = E.import_file(xml)
    @test r["format"] == "bpmn" && r["layout"] isa String
    # the imported text keeps the drawing through the layout sidecar
    @test occursin("bpmndi:BPMNDiagram", E.bpmn(r["text"]; sidecar=r["layout"]))
    dmn = read(joinpath(CONFORMANCE, "dmn-tck", "compliance-level-3", "0001-filter", "0001-filter.dmn"), String)
    t, _ = E.import_dmn(dmn)
    @test startswith(t, "tok 0.7")
    @test E.import_file(dmn)["format"] == "dmn"
    err = try
        E.import_bpmn("<not xml")
        nothing
    catch e
        e
    end
    @test err isa E.TokError && err.kind == "import"
end

@testset "engine: decisions" begin
    text = read(joinpath(CONFORMANCE, "cases", "11-decision.tok"), String)
    @test E.eval_decision(text, "limit", Dict("wiek" => 30, "typ" => "student")) == Dict("limit" => 15)
    @test E.eval_decision(text, "limit", Dict("wiek" => 12, "typ" => "x")) == Dict("limit" => 5)
    err = try
        E.eval_decision(text, "brak")
    catch e
        e
    end
    @test err isa E.TokError && err.kind == "unknown_decision"
    text = read(joinpath(CONFORMANCE, "cases", "61-executable-decisions.tok"), String)
    @test E.eval_decision(text, "naleznosc", Dict("dni-zwloki" => 4, "opłata dzienna" => 1.5)) == 6
end

@testset "engine: errors" begin
    err = try
        E.bpmn("tok 0.1\nstart s\nflow s -> x\n")
    catch e
        e
    end
    @test err isa E.TokError && err.kind == "document" && !isempty(err.diagnostics)
    @test occursin("R00", sprint(showerror, err))
    err = try
        E.call(:tok_fmt, "tok 0.1\n", Dict("lang" => "de"))
    catch e
        e
    end
    @test err isa E.TokError && err.kind == "invalid_argument"
end

@testset "engine: simulation" begin
    text = read(joinpath(ROOT, "examples", "kwerenda-symulacja.tok"), String)
    r = E.simulate(text; cases=40, seed=3, xes=true, csv=true)
    @test r.kpi["cases"]["started"] == 40
    @test startswith(r.report, "Simulation (seed 3): 40 cases started")
    @test length(r.cases) == 40 && !isempty(r.log)
    @test startswith(r.xes, "<?xml") && startswith(r.log_csv, "case,activity")
    @test E.sim_xes(text; cases=40, seed=3) == r.xes
    # the same run as the tok program
    @test E.simulate(text; cases=40, seed=3).kpi == r.kpi
    m = E.sim_model(text)
    @test m["start_event"] == "wplyw" && m["cases"] == 300
    d = E.sim_dist("tri(5m, 10m, 20m)")
    @test d["args"] == [300.0, 600.0, 1200.0]
    @test E.sim_eval("koszt > 150", Dict("koszt" => 200.0)) == (true, true)
    @test E.cli(["--version"]) == 0
end

@testset "engine: calls from several threads" begin
    text = read(joinpath(ROOT, "examples", "kwerenda-symulacja.tok"), String)
    out = Vector{Any}(undef, 8)
    Threads.@threads for k in 1:8
        out[k] = E.simulate(text; cases=20, seed=k, log=false, cases_table=false).kpi["seed"]
    end
    @test out == collect(1:8)
end

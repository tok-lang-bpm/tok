# Command line entry point, shared by bin/tok.jl and the precompile workload.

function cli_usage(io::IO)
    println(io, "usage: tok.jl (fmt|xml|check) FILE [--lang en|pl]")
    return 2
end

"""
    main(args; out=stdout, err=stderr) -> exit code

`fmt FILE` prints the canonical text, `xml FILE` the semantic BPMN XML and
`check FILE` the diagnostics (`--lang pl` for Polish messages).
"""
function main(args::Vector{String}; out::IO=stdout, err::IO=stderr)
    length(args) >= 2 || return cli_usage(err)
    cmd, path = args[1], args[2]
    lang = "en"
    k = findfirst(==("--lang"), args)
    if k !== nothing && k < length(args)
        lang = args[k+1]
    end
    text = read(path, String)
    doc, diags = parse_tok(text)
    errors = count(d -> severity(d) == "error", diags)
    sorted = sort(diags; by=d -> (d.line, d.code))
    if cmd == "fmt"
        print(out, fmt(doc))
    elseif cmd == "xml"
        if errors > 0
            foreach(d -> println(err, format_diag(d, lang, path)), sorted)
            return 1
        end
        print(out, semantic_xml(doc))
    elseif cmd == "check"
        foreach(d -> println(out, format_diag(d, lang, path)), sorted)
        return errors > 0 ? 1 : 0
    else
        return cli_usage(err)
    end
    return 0
end

# Precompile workload: run the main paths once while the package image is built,
# so that the command line starts without compiling the parser and the printer.
let
    example = """
    # przykład
    tok 0.1 "Przykład" executable
    pool p "Basen" process=proc
      lane l "Linia"
        start s "Start" form=f
        user u "Zadanie" form=f
        rule r "Decyzja" decision=d
        xor g "Czy?"
        boundary b "Termin" on=u timer=P7D
        catch c "Wiadomość" message="M" correlation="x"
        script k "Skrypt" expr="a + 1" result=k
        end e "Koniec"
        end e2
        data o "Dokument" state="nowy"
    pool z "Zewnętrzna"
    flow s -> u -> r -> g
    flow g -> c "tak" when="x = 1"
    flow g -> e2 "nie" default
    flow c -> k -> e
    flow b -> e2
    message u -> z "Prośba"
    writes u o
    note n "Uwaga" on=u
    decision d hit=first
      | a: number || b: string || uwagi |
      | < 1 || "x" || mało |
    form f "Formularz"
      text "## Tytuł"
      field a "A" type=number required
    simulate cases=10
    resource rr "Zasób" count=1
    variable a "unif(1, 2)"
    """
    doc, diags = parse_tok(example)
    fmt(doc)
    semantic_xml(doc)
    _, bad = parse_tok("tok 0.1\nprocez x\n\tflow a -> b -> c \"x\"\n")
    for d in bad
        format_diag(d, "en", "x.tok")
        format_diag(d, "pl")
    end
    sort(bad; by=d -> (d.line, d.code))
end

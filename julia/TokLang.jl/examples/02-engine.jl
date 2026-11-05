# The whole language through the Rust engine: checking, export and decisions.
using TokLang

const E = TokLang.Engine

text = read(joinpath(@__DIR__, "limit-wypozyczen.tok"), String)

println("Engine ", E.version())
println("Diagnostics: ", length(E.check(text)))
println("Smallest BPMN conformance class: ", E.bpmn_class(text))

# the files of `tok export`: BPMN with layout, SVG, DMN and forms
mktempdir() do dir
    for (name, content) in E.export_files(text; name="limit")
        write(joinpath(dir, name), content)
        println("  ", name, ": ", length(content), " characters")
    end
end

# a decision of the model, evaluated with its inputs
value = E.eval_decision(text, "limit", Dict("wiek" => 30, "konto" => "student"))
println("Decision `limit`: ", value)

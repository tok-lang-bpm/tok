# The independent core: parsing, the canonical text and the semantic BPMN export.
# It needs no engine library.
using TokLang

text = read(joinpath(@__DIR__, "propozycja-zakupu.tok"), String)

doc, diags = parse_tok(text)
println("Diagnostics of the core: ", length(diags))
foreach(d -> println(format_diag(d)), diags)

println("\nThe first lines of the canonical text:")
foreach(println, first(split(fmt(doc), '\n'), 5))

xml = semantic_xml(doc)
println("\nSemantic BPMN 2.0 XML: ", length(xml), " characters")

# Diagnostic codes, severities and messages in English and Polish.

const ERROR = "error"
const WARNING = "warning"
const NOTE = "note"

const MESSAGES = Dict{String,Tuple{String,String,String}}(
    # tokens
    "T001" => (ERROR, "invalid token: {detail}", "niepoprawny znak lub tekst: {detail}"),
    "T002" => (ERROR, "tab character in indentation; use spaces", "tabulator we wcięciu; używaj spacji"),
    "T003" => (ERROR, "indentation does not match any open block", "wcięcie nie pasuje do żadnego otwartego bloku"),
    # statements
    "P001" => (ERROR, "unknown keyword '{word}'", "nieznane słowo kluczowe „{word}”"),
    "P002" => (ERROR, "'{keyword}' needs an identifier", "„{keyword}” wymaga identyfikatora"),
    "P003" => (ERROR, "'{keyword}' cannot have an indented block", "„{keyword}” nie może mieć wciętego bloku"),
    "P004" => (ERROR, "'{keyword}' is not allowed inside {context}", "„{keyword}” nie może wystąpić w bloku {context}"),
    "P005" => (ERROR, "the document must start with the header 'tok 0.1'", "dokument musi zaczynać się nagłówkiem „tok 0.1”"),
    "P006" => (WARNING, "unknown language version {version}; this tool reads version 0.1",
               "nieznana wersja języka {version}; to narzędzie czyta wersję 0.1"),
    "P007" => (ERROR, "malformed attribute near '{near}'", "niepoprawny atrybut w pobliżu „{near}”"),
    "P008" => (ERROR, "malformed {keyword} line: {detail}", "niepoprawna linia {keyword}: {detail}"),
    "P009" => (ERROR, "malformed decision table row: {detail}", "niepoprawny wiersz tabeli decyzyjnej: {detail}"),
    "P010" => (ERROR, "a second header", "drugi nagłówek dokumentu"),
    # references and structure
    "R001" => (ERROR, "identifier '{id}' is already used on line {other}", "identyfikator „{id}” jest już użyty w linii {other}"),
    "R004" => (ERROR, "identifier '{id}' must use only ASCII letters, digits, '_' and '-'",
               "identyfikator „{id}” może zawierać tylko litery ASCII, cyfry, „_” i „-”"),
    "R011" => (ERROR, "unknown flag '{flag}' on {kind}", "nieznana flaga „{flag}” w elemencie {kind}"),
    "R020" => (ERROR, "a flow line with a label or attributes must contain exactly one arrow",
               "linia flow z etykietą lub atrybutami musi mieć dokładnie jedną strzałkę"),
    # decisions
    "D002" => (ERROR, "rule has {got} cells, the header has {expected}", "reguła ma {got} komórek, a nagłówek {expected}"),
    "D004" => (ERROR, "header cell '{cell}' must be 'name: type' with type string, number, boolean or date",
               "komórka nagłówka „{cell}” musi mieć postać „nazwa: typ” (string, number, boolean, date)"),
)

"Diagnostic codes that every core implementation reports (conformance level core)."
const CORE_CODES = Set(["T001", "T002", "T003", "P001", "P002", "P003", "P004", "P005", "P006", "P007",
                        "P008", "P009", "P010", "R001", "R004", "R020", "D002", "D004"])

struct Diag
    code::String
    line::Int
    id::Union{Nothing,String}
    args::Dict{String,String}
end

Diag(code, line, id=nothing; kw...) =
    Diag(String(code), Int(line), id === nothing ? nothing : String(id),
         Dict{String,String}(String(k) => string(v) for (k, v) in kw))

severity(d::Diag) = haskey(MESSAGES, d.code) ? MESSAGES[d.code][1] : ERROR

function message(d::Diag, lang::AbstractString="en")
    haskey(MESSAGES, d.code) || return d.code
    template = lang == "en" ? MESSAGES[d.code][2] : MESSAGES[d.code][3]
    out = template
    for (k, v) in d.args
        out = replace(out, "{" * k * "}" => v)
    end
    return out
end

function format_diag(d::Diag, lang::AbstractString="en", path=nothing)
    sev = lang == "en" ? Dict(ERROR => "error", WARNING => "warning", NOTE => "note") :
                         Dict(ERROR => "błąd", WARNING => "ostrzeżenie", NOTE => "uwaga")
    where_ = path !== nothing ? "$(path):$(d.line)" : (lang == "en" ? "line $(d.line)" : "linia $(d.line)")
    return "$(where_): $(sev[severity(d)]) $(d.code): $(message(d, lang))"
end

# Canonical form of the statements of language 0.7: DMN types with components, input
# data, knowledge sources, business knowledge models and decisions with their boxed
# expressions.

# keys of language 0.7 whose values are always quoted (lists, expressions and texts)
const QUOTED_07 = ("requires", "params", "with", "question", "answers", "uri", "allowed", "expr", "default")

function tail07(el::Elem, keys, flags)
    parts = String[]
    for part in tail(el, keys, flags)
        k = findfirst('=', part)
        if k !== nothing
            key, v = part[1:prevind(part, k)], part[nextind(part, k):end]
            if key in QUOTED_07 && !startswith(v, "\"")
                part = key * "=" * q(v)
            end
        end
        push!(parts, part)
    end
    return parts
end

isnameword(c::Char) = ('A' <= c <= 'Z') || ('a' <= c <= 'z') || ('0' <= c <= '9') || c in "_-.:/+" || c > '\x7f'

"A name as a token: a word when it reads back as one, otherwise a quoted string."
function name_token(n::AbstractString)
    word = !isempty(n) && (('A' <= n[1] <= 'Z') || ('a' <= n[1] <= 'z') || n[1] == '_' || n[1] > '\x7f') &&
           all(isnameword, n)
    return (word && !occursin("->", n)) ? String(n) : q(n)
end

header_cell(name, typ, values) = values === nothing ? "$(name): $(typ)" : "$(name): $(typ) = $(values)"

has_table(b::Elem) = b.header_done || !isempty(b.inputs) || !isempty(b.outputs) || !isempty(b.rules)

"The components of a DMN type, indented below it."
function components!(pr::Printer, list::Vector{Elem}, indent::Int)
    for c in list
        emit!(pr, indent, join(["item", name_token(c.id), tail07(c, COMPONENT_KEYS, ITEM_FLAGS)...], " "), c)
        components!(pr, c.items, indent + 1)
    end
end

"The decision requirements graph and the decisions of a 0.7 document."
function drg!(pr::Printer)
    d = pr.doc
    if !isempty(d.inputdata) || !isempty(d.sources)
        blank!(pr)
        for b in ovalues(d.inputdata)
            emit!(pr, 0, join(["inputdata", b.id, tail07(b, INPUTDATA_KEYS, ())...], " "), b)
        end
        for b in ovalues(d.sources)
            emit!(pr, 0, join(["source", b.id, tail07(b, SOURCE_KEYS, ())...], " "), b)
        end
    end
    for b in ovalues(d.bkms)
        blank!(pr)
        boxed!(pr, b, 0, join(["bkm", b.id, tail07(b, BKM_KEYS, ())...], " "))
    end
    for dec in ovalues(d.decisions)
        blank!(pr)
        boxed!(pr, dec, 0, join(["decision", dec.id, tail07(dec, DECISION_KEYS_07, ())...], " "))
    end
end

function boxed!(pr::Printer, b::Elem, indent::Int, head::String)
    emit!(pr, indent, head, b)
    has_table(b) && table07!(pr, b, indent + 1)
    for line in b.items
        parts = [line.kind]
        line.kind == "entry" && push!(parts, name_token(line.id))
        append!(parts, tail07(line, BOX_LINE_KEYS, ()))
        boxed!(pr, line, indent + 1, join(parts, " "))
    end
end

"Rows of a decision table or a relation, columns aligned."
function table07!(pr::Printer, dec::Elem, indent::Int)
    pad = "  "^indent
    hin = [header_cell(a, b, get(dec.input_values, k, nothing)) for (k, (a, b)) in enumerate(dec.inputs)]
    hout = [header_cell(a, b, get(dec.output_values, k, nothing)) for (k, (a, b)) in enumerate(dec.outputs)]
    ann = !dec.relation &&
          (any(r -> r.annotation !== nothing && !isempty(r.annotation), dec.rules) || dec.has_annotations)
    rows = Any[(hin, hout, ann ? something(dec.annotation_title, "") : nothing, nothing)]
    for r in dec.rules
        push!(rows, (r.inputs, r.outputs, ann ? something(r.annotation, "") : nothing, r))
    end
    nin = length(dec.inputs)
    ncol = nin + length(dec.outputs)
    widths = zeros(Int, ncol)
    awidth = 0
    for (ins, outs, a, _) in rows
        for (k, c) in enumerate(vcat(ins, outs))
            k <= ncol && (widths[k] = max(widths[k], length(c)))
        end
        a !== nothing && (awidth = max(awidth, length(a)))
    end
    for (ins, outs, a, r) in rows
        cells = vcat(ins, outs)
        cell(k) = k <= length(cells) ? cells[k] : ""
        left = join([ljust(cell(k), widths[k]) for k in 1:nin], " | ")
        if dec.relation
            text = "| $(left) |"
        else
            right = join([ljust(cell(k), widths[k]) for k in nin+1:ncol], " | ")
            text = "| $(left) || $(right) |"
        end
        if ann
            text *= "| " * ljust(something(a, ""), awidth) * " |"
        end
        if r !== nothing
            for c in r.comments
                push!(pr.out, comment_line(pad, c))
            end
            if r.trailing !== nothing && !isempty(r.trailing)
                text *= "  # " * r.trailing
            end
        end
        push!(pr.out, pad * pyrstrip(text))
    end
end

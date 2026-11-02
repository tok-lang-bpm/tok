# Statements of language 0.7: input data, knowledge sources, business knowledge models,
# decisions with boxed expressions (`entry`, `result`, `element`, tables and relations)
# and DMN types with components.

"""
`name: type` or `name: type = values` of a header cell of language 0.7: the name, the
type and the unary tests of the values (`nothing` without `=`). `nothing` when malformed.
"""
function split_header_cell_07(cell::AbstractString)
    chars = collect(cell)
    in_str = false
    eq = nothing
    i = 1
    while i <= length(chars)
        c = chars[i]
        if c == '\\' && in_str
            i += 1
        elseif c == '"'
            in_str = !in_str
        elseif c == '=' && !in_str
            eq = i
            break
        end
        i += 1
    end
    if eq === nothing
        head, values = String(cell), nothing
    else
        head, values = String(chars[1:eq-1]), pystrip(String(chars[eq+1:end]))
    end
    k = findlast(':', head)
    k === nothing && return nothing
    name = pystrip(head[1:prevind(head, k)])
    typ = pystrip(head[nextind(head, k):end])
    (isempty(name) || isempty(typ) || values == "") && return nothing
    return (name, typ, values)
end

"`inputdata` and `source` (language 0.7): elements of the requirements graph without a block."
function st_drg_element!(p::Parser, line::Line, kind::String)::StResult
    id = need_id(p, line)
    id === nothing && return noresult(kind)
    label, attrs, flags, _ = parse_tail(p, line.tokens, 3, line.number)
    e = Elem(kind, id; label=label, attrs=attrs, flags=flags, line=line.number)
    take_comments!(p, e, line)
    if register!(p, e, line.number)
        (kind == "inputdata" ? p.doc.inputdata : p.doc.sources)[id] = e
    end
    return (e, false, nothing)
end

"`decision` of language 0.7 and `bkm`: a box whose block holds its logic."
function st_box_root!(p::Parser, line::Line, bkm::Bool)::StResult
    kind = bkm ? "bkm" : "decision"
    id = need_id(p, line)
    id === nothing && return noresult(kind)
    label, attrs, flags, _ = parse_tail(p, line.tokens, 3, line.number)
    d = Elem(kind, id; label=label, attrs=attrs, flags=flags, line=line.number)
    take_comments!(p, d, line)
    register!(p, d, line.number) || return (d, true, nothing)
    (bkm ? p.doc.bkms : p.doc.decisions)[id] = d
    # the scope of a box context is the identifier of its decision or model
    return (d, true, Ctx("box"; scope=id, owner=d))
end

"""
`entry NAME …`, `result …` and `element …` in the block of a box. The name of an entry
is not a document identifier.
"""
function st_box_line!(p::Parser, line::Line, ctx::Ctx, kind::String)::StResult
    toks = line.tokens
    name = nothing
    start = 2
    if kind == "entry"
        if length(toks) >= 2 && toks[2].kind in (WORD, STRING) &&
           !(toks[2].kind == WORD && length(toks) >= 3 && toks[3].kind == EQUALS)
            name = toks[2].value
            start = 3
        else
            err!(p, "P002", line.number; keyword=kind)
            return noresult(kind)
        end
    end
    _, attrs, flags, _ = parse_tail(p, toks, start, line.number; allow_label=false)
    e = Elem(kind, name; attrs=attrs, flags=flags, line=line.number)
    take_comments!(p, e, line)
    push!(ctx.owner.items, e)
    return (e, true, Ctx("box"; scope=ctx.scope, owner=e))
end

"A row of a decision table or a relation in the block of a box (language 0.7)."
function row07!(p::Parser, line::Line, dec::Elem, root)
    cells = split_row(line.tokens[1].value)
    if cells === nothing
        err!(p, "P009", line.number; detail="unbalanced quotes")
        return
    end
    groups = [String[]]
    for c in cells
        if c == ""
            push!(groups, String[])
        else
            push!(groups[end], pystrip(c))
        end
    end
    comments = p.pending
    p.pending = String[]
    relation_header = !dec.header_done && length(groups) == 1
    if length(groups) > 3 || (dec.header_done && !dec.relation && length(groups) < 2)
        p.pending = comments
        err!(p, "P009", line.number; detail="expected inputs || outputs [|| annotation]")
        return
    end
    annotation() = join([x for x in groups[3] if !isempty(x)], " ")
    if !dec.header_done
        dec.header_done = true
        dec.relation = relation_header
        errs = String[]
        for cell in groups[1]
            hc = split_header_cell_07(cell)
            if hc === nothing
                push!(errs, cell)
            else
                push!(dec.inputs, (hc[1], hc[2]))
                push!(dec.input_values, hc[3])
            end
        end
        if length(groups) >= 2
            for cell in groups[2]
                hc = split_header_cell_07(cell)
                if hc === nothing
                    push!(errs, cell)
                else
                    push!(dec.outputs, (hc[1], hc[2]))
                    push!(dec.output_values, hc[3])
                end
            end
        end
        dec.has_annotations = length(groups) == 3
        dec.annotation_title = length(groups) == 3 ? annotation() : nothing
        for cell in errs
            err!(p, "D009", line.number, root; cell=cell)
        end
        p.pending = comments
        return
    end
    if dec.relation
        if length(groups) != 1 || length(groups[1]) != length(dec.inputs)
            p.pending = comments
            err!(p, "D008", line.number, root; got=sum(length, groups), expected=length(dec.inputs))
            return
        end
        push!(dec.rules, Rule(groups[1], String[], nothing, line.number, comments, line.comment))
        return
    end
    ann = length(groups) == 3 ? annotation() : nothing
    if length(groups[1]) != length(dec.inputs) || length(groups[2]) != length(dec.outputs)
        p.pending = comments
        err!(p, "D002", line.number, root; got=length(groups[1]) + length(groups[2]),
             expected=length(dec.inputs) + length(dec.outputs))
        return
    end
    push!(dec.rules, Rule(groups[1], groups[2], (ann === nothing || isempty(ann)) ? nothing : ann,
                          line.number, comments, line.comment))
end

"`item` of language 0.7: a BPMN item definition, or a DMN type with an optional block of components."
function st_item07!(p::Parser, line::Line)::StResult
    id = need_id(p, line)
    id === nothing && return noresult("item")
    label, attrs, flags, _ = parse_tail(p, line.tokens, 3, line.number)
    e = Elem("item", id; label=label, attrs=attrs, flags=flags, line=line.number)
    take_comments!(p, e, line)
    register!(p, e, line.number) || return (e, true, nothing)
    p.doc.items[id] = e
    return (e, true, Ctx("itemdef"; owner=e))
end

"`item NAME …` in the block of a DMN type: a component (its name is not a document identifier)."
function st_component!(p::Parser, line::Line, ctx::Ctx)::StResult
    toks = line.tokens
    if !(length(toks) >= 2 && toks[2].kind in (WORD, STRING, NUMBER) &&
         !(toks[2].kind == WORD && length(toks) >= 3 && toks[3].kind == EQUALS))
        err!(p, "P002", line.number; keyword="item")
        return noresult("item")
    end
    _, attrs, flags, _ = parse_tail(p, toks, 3, line.number; allow_label=false)
    e = Elem("component", toks[2].value; attrs=attrs, flags=flags, line=line.number)
    take_comments!(p, e, line)
    push!(ctx.owner.items, e)
    return (e, true, Ctx("itemdef"; owner=e))
end

# Event logs: the XES documents of `tok sim --xes` and the CSV files of `tok sim --log`
# (with `--cases-csv`), read into traces of activity instances.

"""
    Instance

One activity instance of an event log: `activity` (the identifier of the activity, or its
name when the log has no identifiers), `label`, `resource`, the times `enqueue`, `start`
and `finish` in seconds since 1970-01-01 (`NaN` when the log does not have them) and
`completed` (`false` for an instance interrupted by a boundary event).
"""
struct Instance
    activity::String
    label::String
    resource::String
    enqueue::Float64
    start::Float64
    finish::Float64
    completed::Bool
end

"""
    Trace

The instances of one case, with its `arrival` time and `end_event` when the log has
them (`NaN` and `""` otherwise).
"""
struct Trace
    case::String
    arrival::Float64
    end_event::String
    instances::Vector{Instance}
end

"""
    EventLog

Traces of an event log, read by [`read_xes`](@ref) or [`read_csv_log`](@ref).
"""
struct EventLog
    traces::Vector{Trace}
    has_end_events::Bool
end

Base.show(io::IO, l::EventLog) =
    print(io, "EventLog(", length(l.traces), " traces, ", sum(length(t.instances) for t in l.traces; init=0),
          " activity instances)")

# --------------------------------------------------------------------- times

days_from_civil(y, m, d) = begin
    y -= m <= 2
    era = fld(y, 400)
    yoe = y - era * 400
    doy = fld(153 * (m + (m > 2 ? -3 : 9)) + 2, 5) + d - 1
    doe = yoe * 365 + fld(yoe, 4) - fld(yoe, 100) + doy
    era * 146097 + doe - 719468
end

"""
    parse_time(text) -> seconds

An ISO 8601 date or date-time (`2027-03-01`, `2027-03-01T09:49:56.984+01:00`) as
seconds since 1970-01-01; a time with a UTC offset is converted to UTC, one without is
taken as it is.
"""
function parse_time(text::AbstractString)
    t = strip(text)
    isempty(t) && return NaN
    m = match(r"^(-?\d{4,})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2})(?::(\d{2})(?:[.,](\d+))?)?)?(Z|[+-]\d{2}:?\d{2})?$", t)
    m === nothing && throw(ArgumentError("not an ISO date-time: $(repr(t))"))
    y, mo, d = parse(Int, m[1]), parse(Int, m[2]), parse(Int, m[3])
    h = m[4] === nothing ? 0 : parse(Int, m[4])
    mi = m[5] === nothing ? 0 : parse(Int, m[5])
    s = m[6] === nothing ? 0 : parse(Int, m[6])
    frac = m[7] === nothing ? 0.0 : parse(Float64, "0." * m[7])
    secs = days_from_civil(y, mo, d) * 86400.0 + h * 3600.0 + mi * 60.0 + s + frac
    off = m[8]
    if off !== nothing && off != "Z"
        sign = off[1] == '-' ? -1 : 1
        digits = replace(off[2:end], ":" => "")
        secs -= sign * (parse(Int, digits[1:2]) * 3600 + parse(Int, digits[3:4]) * 60)
    end
    return secs
end

# ----------------------------------------------------------------------- XES

const ENTITIES = Dict("amp" => "&", "lt" => "<", "gt" => ">", "quot" => "\"", "apos" => "'")

function unescape(s::AbstractString)
    occursin('&', s) || return String(s)
    return replace(s, r"&(#x[0-9a-fA-F]+|#\d+|\w+);" => function (e)
        body = e[2:end-1]
        startswith(body, "#x") && return string(Char(parse(UInt32, body[3:end]; base=16)))
        startswith(body, "#") && return string(Char(parse(UInt32, body[2:end])))
        return get(ENTITIES, body, e)
    end)
end

const ATTR_RE = r"<(string|date|int|float|boolean|id)\s+key=\"([^\"]*)\"\s+value=\"([^\"]*)\"\s*/?>"

"The attributes of one XES element (trace or event) without those of nested events."
function xes_attrs(body::AbstractString)
    d = Dict{String,String}()
    for m in eachmatch(ATTR_RE, body)
        d[unescape(m[2])] = unescape(m[3])
    end
    return d
end

"""
    read_xes(source) -> EventLog

Read an XES event log (IEEE 1849-2016): a file path or the XML text. Activity instances
are formed from the lifecycle transitions of the events: `schedule` gives the enqueue
time, `start` the start, `complete` the end of a completed instance, `ati_abort` and
`withdraw` the end of an interrupted one. Events are grouped into instances by
`concept:instance` (as `tok sim --xes` writes it) or, without it, by activity in order.
The activity is the `activity` attribute when present, else `concept:name`; the trace
attributes `start` and `end_event` give the arrival and the end event of a case.
"""
function read_xes(source::AbstractString)
    text = occursin('<', source) ? source : read(source, String)
    traces = Trace[]
    has_end = false
    for tm in eachmatch(r"<trace>(.*?)</trace>"s, text)
        body = tm[1]
        head = replace(body, r"<event>.*?</event>"s => "")
        ta = xes_attrs(head)
        events = [xes_attrs(em[1]) for em in eachmatch(r"<event>(.*?)</event>"s, body)]
        instances = Instance[]
        open = Dict{String,Int}()          # instance key -> index in `parts`
        parts = Vector{Dict{Symbol,Any}}()
        pending = Dict{String,Vector{Int}}() # activity -> open parts without instance keys
        for ev in events
            act = get(ev, "activity", get(ev, "concept:name", ""))
            trans = lowercase(get(ev, "lifecycle:transition", "complete"))
            time = parse_time(get(ev, "time:timestamp", ""))
            key = get(ev, "concept:instance", "")
            k = if !isempty(key)
                get!(open, key) do
                    push!(parts, Dict{Symbol,Any}(:activity => act))
                    length(parts)
                end
            else
                q = get!(pending, act, Int[])
                if trans == "schedule" || (trans == "start" && (isempty(q) || haskey(parts[q[end]], :start)))
                    push!(parts, Dict{Symbol,Any}(:activity => act))
                    push!(q, length(parts))
                    length(parts)
                elseif isempty(q)
                    push!(parts, Dict{Symbol,Any}(:activity => act))
                    push!(q, length(parts))
                    length(parts)
                else
                    q[1]
                end
            end
            p = parts[k]
            p[:label] = get(ev, "concept:name", act)
            p[:resource] = get(ev, "org:resource", get(p, :resource, ""))
            if trans == "schedule"
                p[:enqueue] = time
            elseif trans == "start"
                p[:start] = time
            elseif trans == "complete" || trans == "ati_abort" || trans == "withdraw" || trans == "pi_abort"
                p[:finish] = time
                p[:completed] = trans == "complete"
                if isempty(key)
                    q = pending[act]
                    idx = findfirst(==(k), q)
                    idx !== nothing && deleteat!(q, idx)
                end
            end
        end
        for p in parts
            haskey(p, :finish) || continue
            push!(instances, Instance(p[:activity], get(p, :label, p[:activity]), get(p, :resource, ""),
                                      get(p, :enqueue, NaN), get(p, :start, NaN), p[:finish], p[:completed]))
        end
        end_event = get(ta, "end_event", "")
        has_end |= haskey(ta, "end_event")
        push!(traces, Trace(get(ta, "concept:name", string(length(traces) + 1)),
                            parse_time(get(ta, "start", "")), end_event, instances))
    end
    return EventLog(traces, has_end)
end

# ----------------------------------------------------------------------- CSV

"Rows of a CSV text (RFC 4180: quoted fields with doubled quotes)."
function csv_rows(text::AbstractString)
    rows = Vector{Vector{String}}()
    row = String[]
    field = IOBuffer()
    quoted = false
    chars = collect(text)
    i = 1
    n = length(chars)
    while i <= n
        c = chars[i]
        if quoted
            if c == '"'
                if i < n && chars[i+1] == '"'
                    write(field, '"')
                    i += 1
                else
                    quoted = false
                end
            else
                write(field, c)
            end
        elseif c == '"'
            quoted = true
        elseif c == ','
            push!(row, String(take!(field)))
        elseif c == '\n' || c == '\r'
            if c == '\r' && i < n && chars[i+1] == '\n'
                i += 1
            end
            push!(row, String(take!(field)))
            push!(rows, row)
            row = String[]
        else
            write(field, c)
        end
        i += 1
    end
    if position(field) > 0 || !isempty(row)
        push!(row, String(take!(field)))
        push!(rows, row)
    end
    return rows
end

function csv_table(source::AbstractString)
    text = occursin('\n', source) ? source : read(source, String)
    rows = csv_rows(text)
    isempty(rows) && return (String[], Vector{Vector{String}}())
    return (rows[1], rows[2:end])
end

"""
    read_csv_log(log; cases=nothing) -> EventLog

Read the event log of `tok sim --log` (columns `case`, `activity`, `label`, `resource`,
`enqueue`, `start`, `end`, `status`), and optionally the table of cases of
`--cases-csv` (columns `case`, `start`, `end`, `end_event`, …), which gives the arrival
and the end event of every case. Each argument is a file path or the CSV text. Without
the table of cases the arrival of a case is the first enqueue time of its instances.
"""
function read_csv_log(log::AbstractString; cases=nothing)
    header, rows = csv_table(log)
    col(name) = (k = findfirst(==(name), header); k === nothing ? throw(ArgumentError("the log has no column '$name'")) : k)
    ci, ai, li, ri, ei, si, fi, st = col("case"), col("activity"), col("label"), col("resource"), col("enqueue"),
                                     col("start"), col("end"), col("status")
    order = String[]
    by_case = Dict{String,Vector{Instance}}()
    for r in rows
        length(r) < length(header) && continue
        c = r[ci]
        haskey(by_case, c) || (push!(order, c); by_case[c] = Instance[])
        push!(by_case[c], Instance(r[ai], r[li], r[ri], parse_time(r[ei]), parse_time(r[si]), parse_time(r[fi]),
                                   r[st] == "completed"))
    end
    arrival = Dict{String,Float64}()
    ends = Dict{String,String}()
    has_end = false
    if cases !== nothing
        ch, crows = csv_table(cases)
        k = findfirst(==("case"), ch)
        ks = findfirst(==("start"), ch)
        ke = findfirst(==("end_event"), ch)
        for r in crows
            length(r) < length(ch) && continue
            ks !== nothing && (arrival[r[k]] = parse_time(r[ks]))
            ke !== nothing && (ends[r[k]] = r[ke])
            haskey(by_case, r[k]) || (push!(order, r[k]); by_case[r[k]] = Instance[])
        end
        has_end = ke !== nothing
    end
    traces = Trace[]
    for c in order
        inst = by_case[c]
        a = get(arrival, c, isempty(inst) ? NaN : minimum(i.enqueue for i in inst))
        push!(traces, Trace(c, a, get(ends, c, ""), inst))
    end
    return EventLog(traces, has_end)
end

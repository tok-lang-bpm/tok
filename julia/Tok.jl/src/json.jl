# A small JSON reader and writer for the messages of the engine's C interface: objects
# become `Dict{String,Any}`, arrays `Vector{Any}`, integers `Int64` (`BigInt` when they do
# not fit), other numbers `Float64`. `NaN`, `Infinity` and `-Infinity`, which the engine
# writes for non-finite floats, are accepted.

module JSON

export parse_json, write_json

struct ParseError <: Exception
    msg::String
end
Base.showerror(io::IO, e::ParseError) = print(io, "invalid JSON: ", e.msg)

mutable struct Reader
    s::Vector{UInt8}
    i::Int
end

@inline peekbyte(r::Reader) = r.i <= length(r.s) ? r.s[r.i] : 0x00

function skipws(r::Reader)
    while r.i <= length(r.s) && r.s[r.i] in (0x20, 0x09, 0x0a, 0x0d)
        r.i += 1
    end
end

function expect(r::Reader, word::String)
    n = ncodeunits(word)
    if r.i + n - 1 <= length(r.s) && String(r.s[r.i:r.i+n-1]) == word
        r.i += n
        return
    end
    throw(ParseError("expected '$word' at byte $(r.i)"))
end

function read_value(r::Reader)
    skipws(r)
    c = peekbyte(r)
    if c == UInt8('{')
        r.i += 1
        d = Dict{String,Any}()
        skipws(r)
        if peekbyte(r) == UInt8('}')
            r.i += 1
            return d
        end
        while true
            skipws(r)
            peekbyte(r) == UInt8('"') || throw(ParseError("expected a key at byte $(r.i)"))
            k = read_string(r)
            skipws(r)
            peekbyte(r) == UInt8(':') || throw(ParseError("expected ':' at byte $(r.i)"))
            r.i += 1
            d[k] = read_value(r)
            skipws(r)
            c = peekbyte(r)
            r.i += 1
            c == UInt8(',') && continue
            c == UInt8('}') && return d
            throw(ParseError("expected ',' or '}' at byte $(r.i - 1)"))
        end
    elseif c == UInt8('[')
        r.i += 1
        a = Any[]
        skipws(r)
        if peekbyte(r) == UInt8(']')
            r.i += 1
            return a
        end
        while true
            push!(a, read_value(r))
            skipws(r)
            c = peekbyte(r)
            r.i += 1
            c == UInt8(',') && continue
            c == UInt8(']') && return a
            throw(ParseError("expected ',' or ']' at byte $(r.i - 1)"))
        end
    elseif c == UInt8('"')
        return read_string(r)
    elseif c == UInt8('t')
        expect(r, "true")
        return true
    elseif c == UInt8('f')
        expect(r, "false")
        return false
    elseif c == UInt8('n')
        expect(r, "null")
        return nothing
    elseif c == UInt8('N')
        expect(r, "NaN")
        return NaN
    elseif c == UInt8('I')
        expect(r, "Infinity")
        return Inf
    else
        return read_number(r)
    end
end

function read_number(r::Reader)
    start = r.i
    if peekbyte(r) == UInt8('-')
        r.i += 1
        if peekbyte(r) == UInt8('I')
            expect(r, "Infinity")
            return -Inf
        end
    end
    isfloat = false
    while r.i <= length(r.s)
        c = r.s[r.i]
        if UInt8('0') <= c <= UInt8('9') || c == UInt8('-') || c == UInt8('+')
            r.i += 1
        elseif c == UInt8('.') || c == UInt8('e') || c == UInt8('E')
            isfloat = true
            r.i += 1
        else
            break
        end
    end
    r.i > start || throw(ParseError("unexpected character at byte $start"))
    text = String(r.s[start:r.i-1])
    if isfloat
        v = tryparse(Float64, text)
        v === nothing && throw(ParseError("invalid number '$text'"))
        return v
    end
    v = tryparse(Int64, text)
    v !== nothing && return v
    b = tryparse(BigInt, text)
    b === nothing && throw(ParseError("invalid number '$text'"))
    return b
end

function hex4(r::Reader)
    r.i + 3 <= length(r.s) || throw(ParseError("truncated \\u escape"))
    v = parse(UInt32, String(r.s[r.i:r.i+3]); base=16)
    r.i += 4
    return v
end

function read_string(r::Reader)
    r.i += 1  # opening quote
    io = IOBuffer()
    while true
        r.i <= length(r.s) || throw(ParseError("unterminated string"))
        c = r.s[r.i]
        r.i += 1
        if c == UInt8('"')
            return String(take!(io))
        elseif c == UInt8('\\')
            e = r.s[r.i]
            r.i += 1
            if e == UInt8('n')
                write(io, '\n')
            elseif e == UInt8('t')
                write(io, '\t')
            elseif e == UInt8('r')
                write(io, '\r')
            elseif e == UInt8('b')
                write(io, '\b')
            elseif e == UInt8('f')
                write(io, '\f')
            elseif e == UInt8('u')
                u = hex4(r)
                if 0xd800 <= u <= 0xdbff && r.i + 1 <= length(r.s) && r.s[r.i] == UInt8('\\') &&
                   r.s[r.i+1] == UInt8('u')
                    r.i += 2
                    lo = hex4(r)
                    u = 0x10000 + ((u - 0xd800) << 10) + (lo - 0xdc00)
                end
                write(io, Char(u))
            else
                write(io, e)  # \" \\ \/
            end
        else
            write(io, c)
        end
    end
end

"""
    parse_json(text) -> value

Read one JSON value.
"""
function parse_json(text::AbstractString)
    r = Reader(Vector{UInt8}(codeunits(String(text))), 1)
    v = read_value(r)
    skipws(r)
    r.i > length(r.s) || throw(ParseError("trailing characters at byte $(r.i)"))
    return v
end

function write_string(io::IO, s::AbstractString)
    write(io, '"')
    for c in s
        if c == '"'
            write(io, "\\\"")
        elseif c == '\\'
            write(io, "\\\\")
        elseif c == '\n'
            write(io, "\\n")
        elseif c == '\r'
            write(io, "\\r")
        elseif c == '\t'
            write(io, "\\t")
        elseif c < ' '
            print(io, "\\u", string(UInt32(c); base=16, pad=4))
        else
            write(io, c)
        end
    end
    write(io, '"')
end

write_value(io::IO, ::Nothing) = write(io, "null")
write_value(io::IO, ::Missing) = write(io, "null")
write_value(io::IO, b::Bool) = write(io, b ? "true" : "false")
write_value(io::IO, x::Integer) = print(io, x)
function write_value(io::IO, x::AbstractFloat)
    # a float keeps its decimal point, so the engine reads it as a float
    isfinite(x) || return write(io, "null")
    t = string(Float64(x))
    print(io, occursin(r"[.eE]", t) ? t : t * ".0")
end
write_value(io::IO, s::AbstractString) = write_string(io, s)
write_value(io::IO, s::Symbol) = write_string(io, String(s))
function write_value(io::IO, d::AbstractDict)
    write(io, '{')
    first = true
    for (k, v) in d
        first || write(io, ',')
        first = false
        write_string(io, string(k))
        write(io, ':')
        write_value(io, v)
    end
    write(io, '}')
end
function write_value(io::IO, nt::NamedTuple)
    write(io, '{')
    first = true
    for (k, v) in pairs(nt)
        first || write(io, ',')
        first = false
        write_string(io, String(k))
        write(io, ':')
        write_value(io, v)
    end
    write(io, '}')
end
function write_value(io::IO, a::Union{AbstractVector,Tuple})
    write(io, '[')
    for (k, v) in enumerate(a)
        k > 1 && write(io, ',')
        write_value(io, v)
    end
    write(io, ']')
end
write_value(io::IO, x) = throw(ArgumentError("cannot write a value of type $(typeof(x)) as JSON"))

"""
    write_json(value) -> String

Compact JSON text of a value: dictionaries and named tuples are objects, vectors and
tuples arrays, `nothing` and `missing` `null`; floats keep a decimal point.
"""
write_json(v) = (io = IOBuffer(); write_value(io, v); String(take!(io)))

end # module JSON

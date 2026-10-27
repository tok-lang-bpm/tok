# Line-based tokenizer: text -> lines of tokens (spec §2).

const WORD = "WORD"
const NUMBER = "NUMBER"
const STRING = "STRING"
const ARROW = "ARROW"
const EQUALS = "EQUALS"
const ROW = "ROW"

struct Token
    kind::String
    value::String
    col::Int
end

struct Line
    number::Int
    indent::Int
    tokens::Vector{Token}
    comment::Union{Nothing,String}
    standalone::Bool
end

const WORD_START = Set(collect("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_"))
const WORD_CHARS = union(WORD_START, Set(collect("0123456789-.:/+")))
const DIGITS = Set(collect("0123456789"))

# Lines end with \r\n, \r or \n (spec §2).
splitlines(text::AbstractString) = split(replace(text, "\r\n" => "\n", "\r" => "\n"), '\n')

# Python str.strip() whitespace (str.isspace): Unicode spaces plus the separators \x1c–\x1f.
pyspace(c::Char) = isspace(c) || c in ('\x1c', '\x1d', '\x1e', '\x1f')
pystrip(s::AbstractString) = String(strip(pyspace, s))
pyrstrip(s::AbstractString) = String(rstrip(pyspace, s))

struct LexError <: Exception
    code::String
    message::String
end

"Tokenize a document. Return (lines, errors) where errors are (code, line, message) tuples."
function tokenize(text::AbstractString)
    lines = Line[]
    errors = Tuple{String,Int,String}[]
    if startswith(text, "﻿")
        text = text[nextind(text, 1):end]
    end
    for (number, raw0) in enumerate(splitlines(text))
        raw = pyrstrip(raw0)
        isempty(pystrip(raw)) && continue
        stripped = lstrip(==(' '), raw)
        lead = length(raw) - length(stripped)
        stripped = String(stripped)
        if startswith(stripped, "\t")
            push!(errors, ("T002", number, "tab character in indentation"))
            stripped = String(lstrip(c -> c == '\t' || c == ' ', stripped))
        end
        indent = lead
        if startswith(stripped, "#")
            push!(lines, Line(number, indent, Token[], pystrip(chop_first(stripped)), true))
            continue
        end
        if startswith(stripped, "|")
            body, comment = split_row_comment(stripped)
            push!(lines, Line(number, indent, [Token(ROW, body, indent)], comment, false))
            continue
        end
        try
            tokens, comment = scan(stripped, indent)
            push!(lines, Line(number, indent, tokens, comment, false))
        catch ex
            ex isa LexError || rethrow()
            push!(errors, (ex.code, number, ex.message))
        end
    end
    return lines, errors
end

chop_first(s::AbstractString) = isempty(s) ? "" : String(s[nextind(s, 1):end])

function split_row_comment(s::String)
    c = collect(s)
    in_str = false
    i = 1
    n = length(c)
    while i <= n
        ch = c[i]
        if ch == '\\' && in_str
            i += 2
            continue
        end
        if ch == '"'
            in_str = !in_str
        elseif ch == '#' && !in_str
            return pyrstrip(String(c[1:i-1])), pystrip(String(c[i+1:end]))
        end
        i += 1
    end
    return s, nothing
end

isword(ch::Char) = ch in WORD_CHARS || ch > '\x7f'

function scan(s::String, offset::Int)
    c = collect(s)
    n = length(c)
    tokens = Token[]
    i = 1
    while i <= n
        ch = c[i]
        col = offset + i
        if ch == ' '
            i += 1
        elseif ch == '#'
            return tokens, pystrip(String(c[i+1:end]))
        elseif ch == '"'
            j = i + 1
            buf = Char[]
            while true
                j > n && throw(LexError("T001", "unterminated string"))
                cj = c[j]
                if cj == '\\' && j + 1 <= n && c[j+1] in ('"', '\\', 'n')
                    push!(buf, c[j+1] == 'n' ? '\n' : c[j+1])
                    j += 2
                elseif cj == '"'
                    break
                else
                    push!(buf, cj)
                    j += 1
                end
            end
            push!(tokens, Token(STRING, String(buf), col))
            i = j + 1
        elseif ch == '-' && i + 1 <= n && c[i+1] == '>'
            push!(tokens, Token(ARROW, "->", col))
            i += 2
        elseif ch == '='
            push!(tokens, Token(EQUALS, "=", col))
            i += 1
        elseif ch in DIGITS || (ch == '-' && i + 1 <= n && c[i+1] in DIGITS)
            j = i + 1
            while j <= n && c[j] in DIGITS
                j += 1
            end
            if j <= n && c[j] == '.' && j + 1 <= n && c[j+1] in DIGITS
                j += 1
                while j <= n && c[j] in DIGITS
                    j += 1
                end
            end
            if j <= n && c[j] in WORD_CHARS && c[j] != '-'
                while j <= n && (c[j] in WORD_CHARS || c[j] in DIGITS)
                    j += 1
                end
                push!(tokens, Token(WORD, String(c[i:j-1]), col))
            elseif j <= n && c[j] == '-' && j + 1 <= n && c[j+1] in DIGITS
                while j <= n && (c[j] in WORD_CHARS || c[j] in DIGITS)
                    j += 1
                end
                push!(tokens, Token(WORD, String(c[i:j-1]), col))
            else
                push!(tokens, Token(NUMBER, String(c[i:j-1]), col))
            end
            i = j
        elseif ch in WORD_START || ch > '\x7f'
            j = i + 1
            while j <= n && isword(c[j])
                if c[j] == '-' && j + 1 <= n && c[j+1] == '>'
                    break
                end
                j += 1
            end
            push!(tokens, Token(WORD, String(c[i:j-1]), col))
            i = j
        else
            throw(LexError("T001", "unexpected character " * pyrepr_char(ch)))
        end
    end
    return tokens, nothing
end

# repr() of a one-character Python string, enough for diagnostics.
function pyrepr_char(ch::Char)
    ch == '\t' && return "'\\t'"
    ch == '\'' && return "\"'\""
    ch == '\\' && return "'\\\\'"
    return "'" * string(ch) * "'"
end

# Insertion-ordered dictionary with string keys. Assigning an existing key
# keeps its position (the behaviour of Python dicts, on which the reference
# implementation relies).

mutable struct ODict{V}
    keys::Vector{String}
    map::Dict{String,V}
end

ODict{V}() where {V} = ODict{V}(String[], Dict{String,V}())

Base.haskey(d::ODict, k::AbstractString) = haskey(d.map, k)
Base.getindex(d::ODict, k::AbstractString) = d.map[k]
Base.get(d::ODict, k, default) = (k isa AbstractString && haskey(d.map, k)) ? d.map[k] : default
Base.length(d::ODict) = length(d.keys)
Base.isempty(d::ODict) = isempty(d.keys)

function Base.setindex!(d::ODict{V}, v, k::AbstractString) where {V}
    if !haskey(d.map, k)
        push!(d.keys, String(k))
    end
    d.map[String(k)] = v
    return v
end

"Values in insertion order."
ovalues(d::ODict) = [d.map[k] for k in d.keys]

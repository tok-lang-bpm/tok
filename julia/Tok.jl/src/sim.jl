"""
    Tok.Sim

The native discrete-event simulator of Tok.jl, for large experiments: many scenarios,
many replications, parallel runs on the threads of Julia. The model comes from the Rust
engine ([`model`](@ref) reads a document with [`Engine.sim_model`](@ref)), and so do the
conditions of flows, which the engine evaluates as its own simulator does. The
semantics are those of the engine's simulator for every element the native simulator
runs; a model with other features ([`unsupported`](@ref)) is rejected with
[`UnsupportedModel`](@ref) and runs in the engine.

```julia
m = Tok.Sim.model(read("kwerenda-symulacja.tok", String))
r = Tok.Sim.simulate(m; seed=7)
e = Tok.Sim.experiment([m, Tok.Sim.scenario(m; name="3 archivists", resources=Dict("archiwisci" => 3))];
                       replications=100)
Tok.Sim.summarize(e)
```
"""
module Sim

using ..Engine
using ..JSON: write_json
using ..Stats: normal_quantile

export SimModel, SimError, UnsupportedModel, Dist, Run, Experiment

include("sim/dist.jl")
include("sim/model.jl")
include("sim/run.jl")
include("sim/result.jl")

end # module Sim

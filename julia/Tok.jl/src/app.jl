# The `tok` application for Julia users: the command line of the engine, the commands of
# the independent core, and two commands of Julia: experiments with the native simulator
# and the calibration on event logs.

const APP_HELP = """
usage: tok COMMAND ...

The commands of the tok program (through the engine):
  check, fmt, bpmn, svg, export, import, sim, eval, cheatsheet, --version

Commands of Tok.jl:
  experiment FILE [--replications N] [--seed N] [--cases N] [--resource ID=COUNT ...] [--json]
                 replications of the native simulator, in parallel on the threads of Julia;
                 every --resource adds a scenario with another number of performers
  fit FILE LOG [--cases CASES.csv] [--level P] [-o FITTED.tok] [--json]
                 maximum-likelihood calibration on an event log (.xes, or the .csv of
                 tok sim --log with the table of cases of --cases-csv); -o writes the
                 document with the fitted parameters
  core (fmt|xml|check) FILE [--lang en|pl]
                 the independent core of Tok.jl
"""

"Value of an option `--name VALUE` (removed from `args`), or `default`."
function take_option!(args::Vector{String}, names...; default=nothing)
    for name in names
        k = findfirst(==(name), args)
        if k !== nothing
            k < length(args) || throw(ArgumentError("option $name needs a value"))
            v = args[k+1]
            deleteat!(args, k:k+1)
            return v
        end
    end
    return default
end

function take_flag!(args::Vector{String}, name)
    k = findfirst(==(name), args)
    k === nothing && return false
    deleteat!(args, k)
    return true
end

function app_experiment(args::Vector{String}, out::IO)
    reps = parse(Int, take_option!(args, "--replications", "-r"; default="10"))
    seed = take_option!(args, "--seed")
    cases = take_option!(args, "--cases")
    json = take_flag!(args, "--json")
    resources = String[]
    while (r = take_option!(args, "--resource")) !== nothing
        push!(resources, r)
    end
    length(args) == 1 || throw(ArgumentError("experiment needs one FILE"))
    text = read(args[1], String)
    m = Sim.model(text; name="base")
    scenarios = Any[m]
    for r in resources
        id, n = split(r, "=")
        push!(scenarios, Sim.scenario(m; name="$id=$n", resources=Dict(String(id) => parse(Int, n))))
    end
    e = Sim.experiment(scenarios; replications=reps, seed=seed === nothing ? nothing : parse(BigInt, seed),
                       cases=cases === nothing ? nothing : parse(Int, cases))
    rows = Sim.summarize(e)
    if json
        println(out, JSON.write_json(rows))
    else
        println(out, "Native simulation: $(length(e.scenarios)) scenario(s) × $reps replications, $(Threads.nthreads()) thread(s)")
        println(out)
        println(out, rpad("scenario", 20), rpad("indicator", 44), lpad("mean", 14), lpad("± 95%", 12))
        for r in rows
            fmtv(x) = isnan(x) ? "" : string(round(x; sigdigits=5))
            println(out, rpad(r.scenario, 20), rpad(r.indicator, 44), lpad(fmtv(r.mean), 14), lpad(fmtv(r.half_width), 12))
        end
    end
    return 0
end

function app_fit(args::Vector{String}, out::IO)
    cases = take_option!(args, "--cases")
    level = parse(Float64, take_option!(args, "--level"; default="0.95"))
    output = take_option!(args, "-o", "--output")
    json = take_flag!(args, "--json")
    length(args) == 2 || throw(ArgumentError("fit needs FILE and LOG"))
    text = read(args[1], String)
    logpath = args[2]
    log = endswith(lowercase(logpath), ".xes") ? Fit.read_xes(logpath) : Fit.read_csv_log(logpath; cases=cases)
    f = Fit.fit(text, log; level=level)
    rows = Fit.table(f)
    if json
        println(out, JSON.write_json([merge(r, (parameter=String(r.parameter), family=String(r.family),
                                                method=String(r.method))) for r in rows]))
    else
        println(out, "Maximum-likelihood estimates, $(round(Int, 100level)) % intervals ($(length(log.traces)) cases)")
        println(out)
        show_time(r, x) = r.parameter === :p ? string(round(x; digits=4)) : Sim.format_duration(x)
        println(out, rpad("element", 34), rpad("parameter", 16), lpad("estimate", 12), lpad("interval", 26), lpad("n", 7))
        for r in rows
            iv = isnan(r.lower) ? "" : show_time(r, r.lower) * " – " * show_time(r, r.upper)
            println(out, rpad(r.element, 34), rpad("$(r.family) $(r.name)", 16), lpad(show_time(r, r.estimate), 12),
                    lpad(iv, 26), lpad(string(r.n), 7))
        end
        for n in f.notes
            println(out, "note: ", n)
        end
    end
    if output !== nothing
        write(output, Fit.apply(text, f))
        json || println(out, "tok: ", output)
    end
    return 0
end

"""
    app_main(args; out=stdout) -> exit status

The `tok` application of Tok.jl (`julia/app`): `experiment` and `fit` run in Julia,
`core …` runs the independent core, every other command runs the command line of the
engine.
"""
function app_main(args::Vector{String}; out::IO=stdout)
    if isempty(args) || args[1] in ("-h", "--help", "help")
        print(out, APP_HELP)
        return isempty(args) ? 2 : 0
    end
    cmd = args[1]
    rest = args[2:end]
    try
        cmd == "core" && return main(rest; out=out)
        cmd == "experiment" && return app_experiment(rest, out)
        cmd == "fit" && return app_fit(rest, out)
        return Engine.cli(args)
    catch e
        # a closed pipe (`tok experiment … | head`) is not an error worth reporting
        e isa Base.IOError && e.code == Base.UV_EPIPE && return 0
        e isa Union{ArgumentError,Engine.TokError,Sim.SimError,Sim.UnsupportedModel,SystemError} || rethrow()
        println(stderr, "tok: ", sprint(showerror, e))
        return e isa ArgumentError ? 2 : 1
    end
end

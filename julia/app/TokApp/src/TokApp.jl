"""
    TokApp

The `tok` application of Tok.jl, compiled with PackageCompiler (`julia/app/build.jl`).
"""
module TokApp

using Tok

"""
The entry point of the compiled program. The engine library is looked up next to the
program first (the build copies it there), then as Tok.jl looks it up.
"""
function julia_main()::Cint
    if !haskey(ENV, "TOK_LIBTOK")
        bundled = joinpath(Sys.BINDIR, Tok.Engine.LIBNAME)
        isfile(bundled) && (ENV["TOK_LIBTOK"] = bundled)
    end
    try
        return Cint(Tok.app_main(copy(ARGS)))
    catch e
        Base.invokelatest(Base.display_error, e, catch_backtrace())
        return Cint(1)
    end
end

end # module TokApp

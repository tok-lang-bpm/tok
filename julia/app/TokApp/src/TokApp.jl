"""
    TokApp

The `tok` application of TokLang.jl, compiled with PackageCompiler (`julia/app/build.jl`).
"""
module TokApp

using TokLang

"""
The entry point of the compiled program. The engine library is the artifact of `tok_jll`,
which the build bundles into the application.
"""
function julia_main()::Cint
    try
        return Cint(TokLang.app_main(copy(ARGS)))
    catch e
        Base.invokelatest(Base.display_error, e, catch_backtrace())
        return Cint(1)
    end
end

end # module TokApp

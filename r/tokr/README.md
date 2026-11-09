# tokr: tok for R

The R package `tokr` reads, checks, formats, exports and simulates process models written in [tok](https://github.com/tok-lang-bpm/tok), and draws them in the viewer and in R Markdown or Quarto documents. The work is done by the Rust engine of tok, the same one the `tok` command and the Python package use, so the results are identical.

## Install

The package compiles its Rust engine during installation. You need:

- R 4.2 or newer, and on Windows [Rtools](https://cran.r-project.org/bin/windows/Rtools/) for your R version;
- Rust 1.88 or newer from <https://rust-lang.org/tools/install/>; on Windows also run `rustup target add x86_64-pc-windows-gnu` once.

From CRAN:

```r
install.packages("tokr")
```

From R-universe, which also has binary packages for Windows and macOS:

```r
install.packages("tokr", repos = c("https://tok-lang-bpm.r-universe.dev", "https://cloud.r-project.org"))
```

From a release archive `tokr_VERSION.tar.gz` (it contains every Rust crate it needs and installs without network access; `r/build.sh` of the repository builds it):

```r
install.packages("tokr_1.0.4.tar.gz", repos = NULL, type = "source")
```

From a checkout of the repository:

```
R CMD INSTALL r/tokr
```

## Use

The package installs three example models, in Polish: `wypozyczenie.tok` (an interlibrary loan), `propozycja-zakupu.tok` (a proposal to buy a book, with a decision table and forms) and `kwerenda-symulacja.tok` (an archive query with the parameters of a simulation). `system.file()` gives their path. Copy a model into the working directory (`getwd()`, changed with `setwd()` or a project of RStudio) before you edit it, because the folder of the package is replaced at every update:

```r
library(tokr)

list.files(system.file("examples", package = "tokr"))
file.copy(system.file("examples", "wypozyczenie.tok", package = "tokr"), "wypozyczenie.tok")

m <- tok_read("wypozyczenie.tok")
m                                     # the first lines of the model
cat(m$text, sep = "\n")               # the whole text
tok_check(m)                          # diagnostics as a data frame
tok_check(m, lang = "pl")             # messages in Polish
tok_plot(m)                           # diagram in the viewer
tok_svg(m, file = "wypozyczenie.svg") # the diagram saved as a file
tok_bpmn(m, file = "wypozyczenie.bpmn")
tok_export(m, dir = "camunda")        # BPMN, SVG, DMN and forms for Camunda 8
tok_fmt(m)                            # canonical text
tok_import("model.bpmn")              # BPMN 2.0 drawn in Camunda Modeler -> tok
tok_eval(system.file("examples", "propozycja-zakupu.tok", package = "tokr"),
         "tryb-akceptacji", list(rodzaj = "drukowana", cena = 120))   # decision table, as tok eval
```

Every function takes a model read with `tok_read()`, the path of a `.tok` file, or the text of a model. `tok_svg()` and `tok_bpmn()` return the text when `file` is not given, so the console prints the SVG or XML code. `options(tok.lang = "pl")` switches all messages to Polish.

A model typed in the console goes between single quotes, because its labels use double quotes; `writeLines()` saves it as a `.tok` file, and `tok_read()` reads the file again after you edit it:

```r
text <- 'tok 1.0 "Book request"
start asked "Reader asks for a book"
user find "Find the book"
end found "Book found"
flow asked -> find -> found'
tok_check(text)
writeLines(text, "book-request.tok")
tok_fmt(tok_read("book-request.tok"), write = TRUE)   # the file in canonical form
```

The section [Working in the Python and R consoles](https://tok-lang-bpm.github.io/getting-started.html#consoles) of the guide takes a first session further.

## Simulation

`tok_sim()` runs the simulation of `tok sim` on a model with `simulate`, `resource` and `variable` statements (see `docs/simulation.md` in the repository). The same model and seed give the same numbers as `tok sim` and the Python package.

```r
k <- system.file("examples", "kwerenda-symulacja.tok", package = "tokr")
s <- tok_sim(k)                          # cases and seed of the simulate statement
s                                        # the report of tok sim
s <- tok_sim(k, cases = 1000, seed = 3, lang = "pl")
s$kpi$activities                         # count, mean wait and processing per activity
s$kpi$resources                          # utilization per resource
s$cases                                  # one row per case, with the case variables
log <- as.data.frame(s)                  # the event log
```

The event log has one row per activity instance, with the columns of `tok sim --log` (`case`, `activity`, `label`, `resource`, `enqueue`, `start`, `end`, `status`) and the same data under the names process-mining packages use (`case_id`, `activity_id`, `activity_instance_id`, `resource_id`, `schedule`, `start`, `complete`). Times are `POSIXct` in UTC. With bupaR:

```r
bupaR::activitylog(log, case_id = "case_id", activity_id = "activity_id",
                   resource_id = "resource_id",
                   timestamps = c("schedule", "start", "complete"))
```

See `?tok_sim` for every element of the result.

### XES event log

`tok_xes()` writes the event log as an XES document (IEEE 1849-2016), the file of `tok sim --xes`, which process-mining tools such as ProM, pm4py, bupaR and Disco read directly. It takes a result of `tok_sim()` (the run is repeated with the same model, number of cases and seed, so the log is the same) or a model:

```r
tok_xes(s, "kwerenda.xes")                                       # the log of s
tok_xes(k, "kwerenda.xes", cases = 500, seed = 3)
```

Each case is a trace; each activity instance gives the events `schedule`, `start` and `complete` (or `ati_abort` and `withdraw` for instances interrupted by a boundary timer while running or while waiting), with the label, the resource and timestamps with milliseconds. See `?tok_xes`.

## In documents

Loading the package registers a knitr engine. A chunk written as

````
```{tok, fig.cap = "Interlibrary loan"}
tok 0.3 "Interlibrary loan"
start s "Request received"
user check "Check the request"
end e "Request handled"
flow s -> check -> e
```
````

draws the diagram. With `tok.output = "check"` the chunk prints the diagnostics instead, with `"fmt"` the canonical text and with `"bpmn"` the BPMN XML; `tok.export = "dir"` also writes every exported file. See `?eng_tok`.

## Build the release archive

`r/build.sh` (in the repository) copies the engine crates into the package, vendors the external crates and runs `R CMD build`; the archive is written to `dist/`.

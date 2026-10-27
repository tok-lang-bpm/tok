# tokr: tok for R

The R package `tokr` reads, checks, formats and exports process models written in [tok](https://codeberg.org/MarekDeja/tok), and draws them in the viewer and in R Markdown or Quarto documents. The work is done by the Rust engine of tok, the same one the `tok` command and the Python package use, so the results are identical.

## Install

The package compiles its Rust engine during installation. You need:

- R 4.2 or newer, and on Windows [Rtools](https://cran.r-project.org/bin/windows/Rtools/) for your R version;
- Rust 1.99 or newer from <https://www.rust-lang.org/tools/install>; on Windows also run `rustup target add x86_64-pc-windows-gnu` once.

From a release archive `tokr_VERSION.tar.gz` (it contains every Rust crate it needs and installs without network access):

```r
install.packages("tokr_0.4.0.tar.gz", repos = NULL, type = "source")
```

From a checkout of the repository:

```
R CMD INSTALL r/tokr
```

## Use

```r
library(tokr)

m <- tok_read("wypozyczenie.tok")
tok_check(m)                    # diagnostics as a data frame
tok_check(m, lang = "pl")       # messages in Polish
tok_plot(m)                     # diagram in the viewer
tok_export(m, dir = "camunda")  # BPMN, SVG, DMN and forms for Camunda 8
tok_fmt(m)                      # canonical text
tok_import("model.bpmn")        # BPMN 2.0 drawn in Camunda Modeler -> tok
tok_eval("propozycja-zakupu.tok", "tryb-akceptacji",
         list(rodzaj = "drukowana", cena = 120))   # decision table, as tok eval
```

Every function takes a model read with `tok_read()`, the path of a `.tok` file, or the text of a model. `options(tok.lang = "pl")` switches all messages to Polish.

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

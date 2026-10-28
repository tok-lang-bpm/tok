//! `tok-lsp`: the language server of tok on standard input and output.

fn main() {
    if let Some(arg) = std::env::args().nth(1) {
        match arg.as_str() {
            "--version" | "-V" => {
                println!("tok-lsp {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "--stdio" => {}
            _ => {
                println!(
                    "usage: tok-lsp [--stdio | --version]\n\nThe language server of tok (LSP 3.17). \
                     It talks to an editor over standard input and output; see docs/lsp.md."
                );
                std::process::exit(if arg == "--help" || arg == "-h" { 0 } else { 2 });
            }
        }
    }
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let code = tok_lsp::run(stdin.lock(), stdout.lock());
    std::process::exit(code);
}

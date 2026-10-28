//! `tok-mcp`: the MCP server of tok on standard input and output.

fn main() {
    if let Some(arg) = std::env::args().nth(1) {
        match arg.as_str() {
            "--version" | "-V" => {
                println!("tok-mcp {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            _ => {
                println!(
                    "usage: tok-mcp [--version]\n\nThe MCP server of tok (stdio transport). An MCP \
                     client starts it and talks to it over standard input and output; see \
                     docs/mcp.md."
                );
                std::process::exit(if arg == "--help" || arg == "-h" { 0 } else { 2 });
            }
        }
    }
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    std::process::exit(tok_mcp::run(stdin.lock(), stdout.lock()));
}

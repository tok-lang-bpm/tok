//! Language server of the tok process language: LSP 3.17 over standard input and output.
//!
//! The server keeps the open documents in memory and analyses each one with the engine
//! (`tok-core` for parsing, checking and printing, `tok-bpmn` for the BPMN rules), so the
//! editor shows exactly the diagnostics of `tok check` and formats exactly as `tok fmt`.
//! Besides diagnostics and formatting it resolves identifiers (go to definition, find
//! references, highlight, rename), gives the outline, hover texts and completion, and
//! answers the requests `tok/svg`, `tok/bpmn`, `tok/export` and `tok/check` of editor
//! extensions, which go through the same functions as the C interface ([`tok_cli::api`]).
//! `docs/lsp.md` documents the features, settings and requests.
//!
//! [`Server`] handles one message at a time and returns the messages to send, so the tests
//! drive it in process; [`run`] connects it to a byte stream with the `Content-Length`
//! framing of the protocol.

mod analysis;
mod index;
mod keywords;
mod server;
mod text;

use std::io::{BufRead, Write};

pub use server::Server;

/// Read one message: the headers up to an empty line, then `Content-Length` bytes of
/// JSON. `None` at the end of the input.
///
/// # Errors
/// An I/O error, or headers without a valid `Content-Length`.
pub fn read_message(input: &mut impl BufRead) -> std::io::Result<Option<String>> {
    let mut length: Option<usize> = None;
    let mut line = String::new();
    loop {
        line.clear();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let header = line.trim_end_matches(['\r', '\n']);
        if header.is_empty() {
            if length.is_some() {
                break;
            }
            continue;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.trim().eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().ok();
        }
    }
    let Some(length) = length else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "message without a valid Content-Length header",
        ));
    };
    let mut body = vec![0u8; length];
    input.read_exact(&mut body)?;
    String::from_utf8(body)
        .map(Some)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Write one message with its `Content-Length` header.
///
/// # Errors
/// An I/O error of the output.
pub fn write_message(output: &mut impl Write, message: &serde_json::Value) -> std::io::Result<()> {
    let body = message.to_string();
    write!(output, "Content-Length: {}\r\n\r\n{body}", body.len())?;
    output.flush()
}

/// Serve one client: read messages from `input` until `exit` or the end of the input and
/// write the responses and notifications to `output`. Returns the exit status: 0 after
/// `shutdown` and `exit`, 1 otherwise (LSP 3.17, `exit`).
pub fn run(mut input: impl BufRead, mut output: impl Write) -> i32 {
    let mut server = Server::new();
    loop {
        let body = match read_message(&mut input) {
            Ok(Some(body)) => body,
            Ok(None) => return if server.is_shut_down() { 0 } else { 1 },
            Err(e) => {
                eprintln!("tok-lsp: {e}");
                return 1;
            }
        };
        for message in server.handle_text(&body) {
            if let Err(e) = write_message(&mut output, &message) {
                eprintln!("tok-lsp: {e}");
                return 1;
            }
        }
        if let Some(code) = server.exit_code() {
            return code;
        }
    }
}

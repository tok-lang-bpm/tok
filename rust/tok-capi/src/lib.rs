//! C interface of the tok engine (the library `tokengine`, header `include/tok.h`).
//!
//! The interface is small and stable: every function takes UTF-8 C strings (the text of
//! a document and the options as a JSON object) and returns a newly allocated UTF-8 C
//! string holding a JSON object, `{"ok": VALUE}` on success or `{"error": {"kind",
//! "message"}}` on failure, which the caller frees with [`tok_free`]. [`tok_version`]
//! and [`tok_abi_version`] identify the library. No function panics across the
//! boundary. The functions, options and results are documented in `docs/c-api.md`.

mod api;
#[allow(unsafe_code)]
mod ffi;

pub use ffi::{
    ABI_VERSION, tok_abi_version, tok_bpmn, tok_check, tok_cli, tok_eval, tok_export, tok_fmt,
    tok_free, tok_import, tok_sim, tok_sim_dist, tok_sim_eval, tok_sim_model, tok_svg, tok_version,
};

//! The exported C functions. This is the only module of the crate with `unsafe` code:
//! reading the C strings passed in and taking back the strings handed out.
//!
//! Every function catches panics, so none unwinds into the caller; a panic becomes an
//! error of kind `internal`.

use std::ffi::{CStr, CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};

use tok_cli::api::{self, ApiError, Res};

/// Version of the C interface: incremented when a function is removed or changes its
/// parameters or the shape of its result in an incompatible way.
pub const ABI_VERSION: u32 = 1;

const VERSION: &CStr =
    match CStr::from_bytes_with_nul(concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes()) {
        Ok(v) => v,
        Err(_) => panic!("the package version contains a NUL byte"),
    };

/// A string argument: `NULL` is `None`, other pointers must be NUL-terminated UTF-8.
///
/// # Safety
/// `ptr` is `NULL` or points to a NUL-terminated string that stays valid and unchanged
/// during the call.
unsafe fn arg<'a>(ptr: *const c_char, name: &str) -> Result<Option<&'a str>, ApiError> {
    if ptr.is_null() {
        return Ok(None);
    }
    // SAFETY: the caller guarantees a NUL-terminated string valid for the call.
    let bytes = unsafe { CStr::from_ptr(ptr) };
    bytes.to_str().map(Some).map_err(|e| {
        ApiError::new(
            "invalid_argument",
            format!("{name} is not valid UTF-8: {e}"),
        )
    })
}

/// A newly allocated C string for the caller, freed with [`tok_free`].
fn hand_out(text: String) -> *mut c_char {
    let c = CString::new(text).unwrap_or_else(|_| {
        // JSON escapes every control character, so the result never holds a NUL byte
        CString::from(
            c"{\"error\":{\"kind\":\"internal\",\"message\":\"result contains a NUL byte\"}}",
        )
    });
    c.into_raw()
}

fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    }
}

/// Run one call of the interface: read the arguments, call `f` and serialize its result.
///
/// # Safety
/// `input` and `options` are `NULL` or NUL-terminated strings valid during the call.
unsafe fn call(
    input: *const c_char,
    input_name: &str,
    options: *const c_char,
    f: fn(&str, &str) -> Res,
) -> *mut c_char {
    // SAFETY: forwarded from the caller's guarantee.
    let input = unsafe { arg(input, input_name) };
    // SAFETY: forwarded from the caller's guarantee.
    let options = unsafe { arg(options, "options") };
    let result = catch_unwind(AssertUnwindSafe(|| -> Res {
        let Some(input) = input? else {
            return Err(ApiError::new(
                "invalid_argument",
                format!("{input_name} must not be NULL"),
            ));
        };
        f(input, options?.unwrap_or(""))
    }));
    let result = match result {
        Ok(r) => r,
        Err(payload) => Err(ApiError::new(
            "internal",
            format!("internal error: {}", panic_text(payload.as_ref())),
        )),
    };
    let text = catch_unwind(AssertUnwindSafe(|| api::respond(&result))).unwrap_or_else(|_| {
        "{\"error\":{\"kind\":\"internal\",\"message\":\"internal error\"}}".to_string()
    });
    hand_out(text)
}

/// Version of the engine, for example `1.0.0`. The string is static: do not free it.
#[unsafe(no_mangle)]
pub extern "C" fn tok_version() -> *const c_char {
    VERSION.as_ptr()
}

/// Version of the C interface ([`ABI_VERSION`]).
#[unsafe(no_mangle)]
pub extern "C" fn tok_abi_version() -> u32 {
    ABI_VERSION
}

/// Free a string returned by a function of this library. `NULL` is ignored.
///
/// # Safety
/// `s` is `NULL` or a pointer returned by a function of this library that has not been
/// freed yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tok_free(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    // SAFETY: `s` came from `CString::into_raw` in `hand_out` and is freed only once.
    drop(unsafe { CString::from_raw(s) });
}

macro_rules! export {
    ($(#[$doc:meta])* $name:ident, $input:literal, $f:path) => {
        $(#[$doc])*
        ///
        /// # Safety
        /// The arguments are `NULL` or NUL-terminated strings valid during the call; the
        /// result is freed with [`tok_free`].
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            input: *const c_char,
            options: *const c_char,
        ) -> *mut c_char {
            // SAFETY: the caller's guarantee is the one `call` requires.
            unsafe { call(input, $input, options, $f) }
        }
    };
}

export!(
    /// Diagnostics of a document (`tok check`).
    tok_check, "text", api::check
);
export!(
    /// Canonical text of a document (`tok fmt`).
    tok_fmt, "text", api::fmt
);
export!(
    /// BPMN 2.0 XML with diagram layout (`tok bpmn`).
    tok_bpmn, "text", api::bpmn
);
export!(
    /// SVG picture of the diagram (`tok svg`).
    tok_svg, "text", api::svg
);
export!(
    /// Every file of `tok export`, as an object file name → content.
    tok_export, "text", api::export_files
);
export!(
    /// A BPMN 2.0 or DMN file as a tok document (`tok import`).
    tok_import, "xml", api::import_file
);
export!(
    /// The value of a decision (`tok eval`).
    tok_eval, "text", api::eval
);
export!(
    /// A simulation run (`tok sim`).
    tok_sim, "text", api::sim
);
export!(
    /// The simulation model of a document, for simulators outside the engine.
    tok_sim_model, "text", api::sim_model
);
export!(
    /// A distribution of the simulation statements, parsed.
    tok_sim_dist, "text", api::sim_dist
);
export!(
    /// An expression evaluated as the simulator evaluates it.
    tok_sim_eval, "expr", api::sim_eval
);
export!(
    /// The `tok` command line (arguments as a JSON array), writing to the standard
    /// streams of the process.
    tok_cli, "args", api::cli
);

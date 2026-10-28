//! The tok engine as a WebAssembly module for the browser playground (`playground/`).
//!
//! Every function takes the text of a document (or of an XML file) and the options as a
//! JSON object, and returns the JSON text the C interface returns: `{"ok": VALUE}` on
//! success or `{"error": {"kind", "message"}}` on failure. The options and results are
//! those of the C interface (`docs/c-api.md`), because both call [`tok_cli::api`]; the
//! playground and `docs/playground.md` show how a page uses them.
//!
//! A panic of the engine aborts the module (WebAssembly has no unwinding); the panic hook
//! writes its message to the console first, and the page loads a fresh module. The system
//! clock does not exist in a browser, so FEEL `now()` and `today()` read the clock of the
//! page (`Date.now()`).

use tok_cli::api;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn console_error(message: &str);
    #[wasm_bindgen(js_namespace = Date, js_name = now)]
    fn date_now() -> f64;
}

/// Prepare the module (it runs when the page loads it): the panic hook writes the message
/// of a panic to the console of the page, and the clock of FEEL is the clock of the page.
#[wasm_bindgen(start)]
pub fn start() {
    if cfg!(target_family = "wasm") {
        std::panic::set_hook(Box::new(|info| {
            console_error(&format!("tok engine: {info}"));
        }));
        use_page_clock();
    }
}

/// FEEL `now()` and `today()` read `Date.now()` of the page (WebAssembly only).
pub fn use_page_clock() {
    if cfg!(target_family = "wasm") {
        // milliseconds of the page to nanoseconds
        tok_core::pytime::set_clock(|| (date_now() * 1e6) as i128);
    }
}

/// The version of the engine, for example `0.9.0`.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// The one-page summary of the language (`tok cheatsheet`).
#[wasm_bindgen]
pub fn cheatsheet() -> String {
    include_str!("../../../docs/cheatsheet.md").to_string()
}

/// Diagnostics of a document (`tok check`): options `lang`, `level`, `bpmn`, `class`.
#[wasm_bindgen]
pub fn check(text: &str, options: &str) -> String {
    api::respond(&api::check(text, options))
}

/// The canonical text of a document (`tok fmt`): option `lang`.
#[wasm_bindgen]
pub fn fmt(text: &str, options: &str) -> String {
    api::respond(&api::fmt(text, options))
}

/// BPMN 2.0 XML with diagram layout (`tok bpmn`): options `lang`, `layout`, `sidecar`.
#[wasm_bindgen]
pub fn bpmn(text: &str, options: &str) -> String {
    api::respond(&api::bpmn(text, options))
}

/// The SVG picture of the diagram (`tok svg`): options `lang`, `title`, `sidecar`.
#[wasm_bindgen]
pub fn svg(text: &str, options: &str) -> String {
    api::respond(&api::svg(text, options))
}

/// Every file of `tok export` as an object file name → content: options `lang`, `name`,
/// `sidecar`.
#[wasm_bindgen]
pub fn export(text: &str, options: &str) -> String {
    api::respond(&api::export_files(text, options))
}

/// A BPMN 2.0 or DMN file as a tok document (`tok import`): options `format`,
/// `flow_ids`, `layout`.
#[wasm_bindgen(js_name = "import")]
pub fn import_file(xml: &str, options: &str) -> String {
    api::respond(&api::import_file(xml, options))
}

/// The value of a decision (`tok eval`): options `decision`, `values`, `lang`.
#[wasm_bindgen]
pub fn eval(text: &str, options: &str) -> String {
    api::respond(&api::eval(text, options))
}

/// A simulation run (`tok sim`): options `cases`, `seed`, `lang`, `log`, `cases_table`,
/// `csv`, `xes`.
#[wasm_bindgen]
pub fn sim(text: &str, options: &str) -> String {
    api::respond(&api::sim(text, options))
}

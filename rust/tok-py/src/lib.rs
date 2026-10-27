//! Python bindings of the tok engine (module `tok_engine`).
//!
//! The functions mirror the pure-Python package `tok` so that both can be compared on the
//! same inputs. Until the 1.0 registry release the installed `tok-lang` package stays pure
//! Python; this module exists for parity testing.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use tok_core::check::{BpmnIssue, BpmnRules};
use tok_core::diagnostics::{Diag, format_diag};
use tok_core::{Document, dmn, export, forms, import, printer};

/// BPMN rules from the `tok-bpmn` crate (as in the command line).
struct Rules;

impl BpmnRules for Rules {
    fn validate(&self, xml: &str) -> Vec<BpmnIssue> {
        tok_bpmn::validate(xml)
            .unwrap_or_default()
            .into_iter()
            .map(|i| BpmnIssue {
                level: match i.level {
                    tok_bpmn::Level::Error => "błąd",
                    tok_bpmn::Level::Warning => "ostrzeżenie",
                    tok_bpmn::Level::Note => "uwaga",
                }
                .to_string(),
                code: i.code,
                id: i.id,
                message: i.message,
            })
            .collect()
    }
}

/// Universal newlines, like Python's text-mode `open`.
fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// A `ValueError` listing the errors among the diagnostics.
fn errors_to_py(diags: &[Diag]) -> PyErr {
    let text = diags
        .iter()
        .filter(|d| d.is_error())
        .map(|d| format_diag(d, "en", None))
        .collect::<Vec<_>>()
        .join("\n");
    PyValueError::new_err(text)
}

/// Parse only; raise on parser errors.
fn parse_ok(text: &str) -> PyResult<Document> {
    let (doc, diags) = tok_core::parse(&normalize(text));
    if diags.iter().any(Diag::is_error) {
        return Err(errors_to_py(&diags));
    }
    Ok(doc)
}

/// Parse and check with the BPMN rules (as `tok check`); raise on any error.
fn load_ok(text: &str) -> PyResult<Document> {
    let rules = Rules;
    let (doc, diags) = tok_core::load(&normalize(text), "full", Some(&rules));
    if diags.iter().any(Diag::is_error) {
        return Err(errors_to_py(&diags));
    }
    Ok(doc)
}

fn bpmn_with_layout(doc: &Document) -> PyResult<String> {
    tok_bpmn::layout(&export::export_xml(doc, None))
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

fn svg_of(doc: &Document, title: Option<&str>) -> PyResult<String> {
    tok_bpmn::render_svg(&export::export_xml(doc, None), title)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Version of the engine.
#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Diagnostics of a document, as `tok check`: a list of dicts with the keys
/// `code`, `severity`, `line`, `id` and `message`, sorted by line and code.
#[pyfunction]
#[pyo3(signature = (text, lang = "en", level = "full", bpmn = true))]
fn check<'py>(
    py: Python<'py>,
    text: &str,
    lang: &str,
    level: &str,
    bpmn: bool,
) -> PyResult<Bound<'py, PyList>> {
    if lang != "en" && lang != "pl" {
        return Err(PyValueError::new_err(format!(
            "lang must be 'en' or 'pl', not '{lang}'"
        )));
    }
    if !["basic", "analytic", "full"].contains(&level) {
        return Err(PyValueError::new_err(format!(
            "level must be 'basic', 'analytic' or 'full', not '{level}'"
        )));
    }
    let rules = Rules;
    let (_, diags) = tok_core::load(
        &normalize(text),
        level,
        if bpmn { Some(&rules) } else { None },
    );
    let list = PyList::empty(py);
    for d in &diags {
        let item = PyDict::new(py);
        item.set_item("code", &d.code)?;
        item.set_item("severity", d.severity().as_str())?;
        item.set_item("line", d.line)?;
        item.set_item("id", d.id.as_deref())?;
        item.set_item("message", d.message(lang))?;
        list.append(item)?;
    }
    Ok(list)
}

/// Canonical text of a document (`tok fmt`).
#[pyfunction]
fn fmt(text: &str) -> PyResult<String> {
    Ok(printer::fmt(&parse_ok(text)?))
}

/// Semantic BPMN XML without layout, attributes sorted (conformance format).
#[pyfunction]
fn semantic_xml(text: &str) -> PyResult<String> {
    Ok(export::semantic_xml(&parse_ok(text)?, None))
}

/// BPMN XML with diagram layout (`tok bpmn`).
#[pyfunction]
fn bpmn(text: &str) -> PyResult<String> {
    bpmn_with_layout(&load_ok(text)?)
}

/// SVG picture of the diagram (`tok svg`).
#[pyfunction]
#[pyo3(signature = (text, title = None))]
fn svg(text: &str, title: Option<&str>) -> PyResult<String> {
    svg_of(&load_ok(text)?, title)
}

/// Every file `tok export` writes, as a dict file name → content:
/// `NAME.bpmn`, `NAME.svg`, one `.dmn` per decision and one `.form` per form.
#[pyfunction]
#[pyo3(name = "export", signature = (text, name = "model"))]
fn export_files<'py>(py: Python<'py>, text: &str, name: &str) -> PyResult<Bound<'py, PyDict>> {
    let doc = load_ok(text)?;
    let files = PyDict::new(py);
    files.set_item(format!("{name}.bpmn"), bpmn_with_layout(&doc)?)?;
    let svg_name = format!("{name}.svg");
    files.set_item(&svg_name, svg_of(&doc, Some(&svg_name))?)?;
    for dec in doc.decisions.values() {
        files.set_item(
            format!("{}.dmn", dec.base.id()),
            dmn::export_dmn(dec, Some(&doc)),
        )?;
    }
    for form in doc.forms.values() {
        files.set_item(format!("{}.form", form.base.id()), forms::export_form(form))?;
    }
    Ok(files)
}

/// Convert BPMN 2.0 XML to tok (`tok import`): (canonical text, warnings).
#[pyfunction]
#[pyo3(signature = (xml, keep_ids = true))]
fn import_bpmn(xml: &str, keep_ids: bool) -> PyResult<(String, Vec<String>)> {
    let (doc, warnings) =
        import::import_bpmn(xml, keep_ids).map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok((printer::fmt(&doc), warnings))
}

#[pymodule]
fn tok_engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_function(wrap_pyfunction!(check, m)?)?;
    m.add_function(wrap_pyfunction!(fmt, m)?)?;
    m.add_function(wrap_pyfunction!(semantic_xml, m)?)?;
    m.add_function(wrap_pyfunction!(bpmn, m)?)?;
    m.add_function(wrap_pyfunction!(svg, m)?)?;
    m.add_function(wrap_pyfunction!(export_files, m)?)?;
    m.add_function(wrap_pyfunction!(import_bpmn, m)?)?;
    Ok(())
}

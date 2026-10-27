//! Python bindings of the tok engine (module `tok_engine`).
//!
//! The functions mirror the pure-Python package `tok` so that both can be compared on the
//! same inputs. Until the 1.0 registry release the installed `tok-lang` package stays pure
//! Python; this module exists for parity testing.

use pyo3::create_exception;
use pyo3::exceptions::{
    PyException, PyFileNotFoundError, PyIsADirectoryError, PyKeyError, PyMemoryError, PyOSError,
    PyOverflowError, PyPermissionError, PyRuntimeError, PyTypeError, PyValueError,
};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple};

use tok_core::check::{BpmnIssue, BpmnRules};
use tok_core::diagnostics::{Diag, format_diag};
use tok_core::feel_eval::{self as fe, Context, PyValue, Value};
use tok_core::json::Json;
use tok_core::pydec::Decimal;
use tok_core::pytime::{Date, DateTime, Duration};
use tok_core::pyutil::PyError;
use tok_core::{Document, dmn, export, forms, import, printer};
use tok_sim::result::{ActivityKpi, Kpi, ResourceKpi};
use tok_sim::{Param, SimResult};

create_exception!(
    tok_engine,
    FeelError,
    PyException,
    "A FEEL expression that cannot be evaluated (tok.feel.FeelError)."
);
create_exception!(
    tok_engine,
    FeelSyntaxError,
    FeelError,
    "A malformed FEEL expression, F002 (tok.feel.FeelSyntaxError)."
);
create_exception!(
    tok_engine,
    FeelUnsupported,
    FeelError,
    "Valid FEEL outside the subset of tok, F001 (tok.feel.FeelUnsupported)."
);
create_exception!(
    tok_engine,
    DecisionError,
    PyException,
    "A violated hit policy of a decision table (tok.dmn.DecisionError)."
);
create_exception!(
    tok_engine,
    SimError,
    PyException,
    "A model the simulator cannot run, e.g. without a start event (tok.sim.SimError)."
);
create_exception!(
    tok_engine,
    DistError,
    PyException,
    "An invalid distribution (tok.sim.DistError)."
);

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

/// The Python exception that the reference implementation raises for an evaluation error:
/// the classes of `decimal` and the built-in ones are used as they are, the classes of the
/// `tok` package are mirrored by the exceptions of this module.
fn eval_error_to_py(py: Python<'_>, e: PyError) -> PyErr {
    let msg = e.message.clone();
    match e.kind.as_str() {
        "tok.feel.FeelSyntaxError" => FeelSyntaxError::new_err(msg),
        "tok.feel.FeelUnsupported" => FeelUnsupported::new_err(msg),
        "tok.dmn.DecisionError" => DecisionError::new_err(msg),
        "tok.sim.SimError" => SimError::new_err(msg),
        "tok.sim.DistError" => DistError::new_err(msg),
        "ValueError" => PyValueError::new_err(msg),
        "FileNotFoundError" => PyFileNotFoundError::new_err(msg),
        "PermissionError" => PyPermissionError::new_err(msg),
        "IsADirectoryError" => PyIsADirectoryError::new_err(msg),
        "OSError" => PyOSError::new_err(msg),
        "OverflowError" => PyOverflowError::new_err(msg),
        "TypeError" => PyTypeError::new_err(msg),
        "MemoryError" => PyMemoryError::new_err(()),
        kind if kind.starts_with("decimal.") => {
            let made = py
                .import("decimal")
                .and_then(|m| m.getattr(e.class_name()))
                .and_then(|cls| cls.call1((msg.clone(),)));
            match made {
                Ok(exc) => PyErr::from_value(exc),
                Err(err) => err,
            }
        }
        _ => PyRuntimeError::new_err(e.to_string()),
    }
}

fn int_of(obj: &Bound<'_, PyAny>, name: &str) -> PyResult<i64> {
    obj.getattr(name)?.extract::<i64>()
}

fn duration_of(obj: &Bound<'_, PyAny>) -> PyResult<Duration> {
    let us = int_of(obj, "days")? as i128 * 86_400_000_000
        + int_of(obj, "seconds")? as i128 * 1_000_000
        + int_of(obj, "microseconds")? as i128;
    Duration::from_micros(us).map_err(|e| PyOverflowError::new_err(e.message))
}

fn decimal_of(text: &str) -> PyResult<Decimal> {
    Decimal::parse(text).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// `feel.to_feel`: a Python value as a FEEL value.
fn value_from_py(obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    let py = obj.py();
    if obj.is_none() {
        return Ok(Value::Null);
    }
    if obj.is_instance_of::<PyBool>() {
        return Ok(Value::Bool(obj.extract::<bool>()?));
    }
    if obj.is_instance_of::<PyString>() {
        return Ok(Value::Str(obj.extract::<String>()?));
    }
    if obj.is_instance_of::<PyInt>() {
        return Ok(Value::Num(decimal_of(&obj.str()?.to_string())?));
    }
    if obj.is_instance_of::<PyFloat>() {
        return Ok(Value::Num(decimal_of(&obj.repr()?.to_string())?));
    }
    let decimal = py.import("decimal")?;
    if obj.is_instance(&decimal.getattr("Decimal")?)? {
        return Ok(Value::Num(decimal_of(&obj.str()?.to_string())?));
    }
    let datetime = py.import("datetime")?;
    if obj.is_instance(&datetime.getattr("datetime")?)? {
        let date = Date::new(
            int_of(obj, "year")? as i128,
            int_of(obj, "month")? as i128,
            int_of(obj, "day")? as i128,
        )
        .map_err(|e| PyValueError::new_err(e.message))?;
        let off = obj.call_method0("utcoffset")?;
        return Ok(Value::DateTime(DateTime {
            date,
            hour: int_of(obj, "hour")? as u32,
            minute: int_of(obj, "minute")? as u32,
            second: int_of(obj, "second")? as u32,
            microsecond: int_of(obj, "microsecond")? as u32,
            offset: if off.is_none() {
                None
            } else {
                Some(duration_of(&off)?)
            },
        }));
    }
    if obj.is_instance(&datetime.getattr("date")?)? {
        let date = Date::new(
            int_of(obj, "year")? as i128,
            int_of(obj, "month")? as i128,
            int_of(obj, "day")? as i128,
        )
        .map_err(|e| PyValueError::new_err(e.message))?;
        return Ok(Value::Date(date));
    }
    if obj.is_instance(&datetime.getattr("timedelta")?)? {
        return Ok(Value::Duration(duration_of(obj)?));
    }
    if obj.is_instance_of::<PyList>() || obj.is_instance_of::<PyTuple>() {
        let mut items = Vec::new();
        for item in obj.try_iter()? {
            items.push(value_from_py(&item?)?);
        }
        return Ok(Value::List(items));
    }
    if let Ok(dict) = obj.cast::<PyDict>() {
        return Ok(Value::Context(context_from_py(dict)?));
    }
    Err(PyTypeError::new_err(format!(
        "unsupported value type: {}",
        obj.get_type().name()?
    )))
}

fn context_from_py(dict: &Bound<'_, PyDict>) -> PyResult<Context> {
    let mut ctx = Context::new();
    for (k, v) in dict.iter() {
        let key: String = k
            .extract()
            .map_err(|_| PyTypeError::new_err("context keys must be strings"))?;
        ctx.insert(key, value_from_py(&v)?);
    }
    Ok(ctx)
}

fn duration_to_py<'py>(py: Python<'py>, d: &Duration) -> PyResult<Bound<'py, PyAny>> {
    py.import("datetime")?
        .getattr("timedelta")?
        .call1((d.days, d.seconds, d.microseconds))
}

/// A FEEL value as the Python object `feel.py` returns (`decimal.Decimal`, `datetime` …).
fn value_to_py<'py>(py: Python<'py>, v: &Value) -> PyResult<Bound<'py, PyAny>> {
    Ok(match v {
        Value::Null => py.None().into_bound(py),
        Value::Bool(b) => PyBool::new(py, *b).to_owned().into_any(),
        Value::Num(d) => py
            .import("decimal")?
            .getattr("Decimal")?
            .call1((d.to_sci_string(),))?,
        Value::Str(s) => PyString::new(py, s).into_any(),
        Value::Date(d) => py
            .import("datetime")?
            .getattr("date")?
            .call1((d.year, d.month, d.day))?,
        Value::DateTime(t) => {
            let dt = py.import("datetime")?;
            let tz = match &t.offset {
                None => py.None().into_bound(py),
                Some(off) => dt.getattr("timezone")?.call1((duration_to_py(py, off)?,))?,
            };
            dt.getattr("datetime")?.call1((
                t.date.year,
                t.date.month,
                t.date.day,
                t.hour,
                t.minute,
                t.second,
                t.microsecond,
                tz,
            ))?
        }
        Value::Duration(d) => duration_to_py(py, d)?,
        Value::List(items) => {
            let list = PyList::empty(py);
            for x in items {
                list.append(value_to_py(py, x)?)?;
            }
            list.into_any()
        }
        Value::Context(m) => {
            let dict = PyDict::new(py);
            for (k, x) in m {
                dict.set_item(k, value_to_py(py, x)?)?;
            }
            dict.into_any()
        }
    })
}

/// A plain value (`feel.to_python`) as a Python object.
fn plain_to_py<'py>(py: Python<'py>, v: &PyValue) -> PyResult<Bound<'py, PyAny>> {
    Ok(match v {
        PyValue::None => py.None().into_bound(py),
        PyValue::Bool(b) => PyBool::new(py, *b).to_owned().into_any(),
        // through Decimal: int(str) refuses texts longer than 4300 digits
        PyValue::Int(t) => py.import("builtins")?.getattr("int")?.call1((py
            .import("decimal")?
            .getattr("Decimal")?
            .call1((t.as_str(),))?,))?,
        PyValue::Float(f) => PyFloat::new(py, *f).into_any(),
        PyValue::Str(s) => PyString::new(py, s).into_any(),
        PyValue::Duration(d) => duration_to_py(py, d)?,
        PyValue::List(items) => {
            let list = PyList::empty(py);
            for x in items {
                list.append(plain_to_py(py, x)?)?;
            }
            list.into_any()
        }
        PyValue::Dict(pairs) => {
            let dict = PyDict::new(py);
            for (k, x) in pairs {
                dict.set_item(k, plain_to_py(py, x)?)?;
            }
            dict.into_any()
        }
    })
}

/// Evaluate a decision table of a document (`dmn.evaluate_decision`) for the given
/// variables: a dict {output name: value}, `None` when no rule matches, or a list of dicts
/// for hit policy collect. Numbers come back as int or float, dates as ISO text.
/// Raises `ValueError` for a document with errors, `KeyError` for an unknown decision,
/// `DecisionError` for a violated hit policy and the FEEL and arithmetic errors of the
/// reference implementation.
#[pyfunction]
#[pyo3(signature = (text, decision, values = None))]
fn eval_decision<'py>(
    py: Python<'py>,
    text: &str,
    decision: &str,
    values: Option<&Bound<'py, PyDict>>,
) -> PyResult<Bound<'py, PyAny>> {
    let (doc, diags) = tok_core::load(&normalize(text), "full", None);
    if diags.iter().any(Diag::is_error) {
        return Err(errors_to_py(&diags));
    }
    let Some(dec) = doc.decisions.get(decision) else {
        return Err(PyKeyError::new_err(decision.to_string()));
    };
    let ctx = match values {
        Some(v) => context_from_py(v)?,
        None => Context::new(),
    };
    let result = dmn::evaluate_decision(dec, &ctx).map_err(|e| eval_error_to_py(py, e))?;
    plain_to_py(py, &result)
}

/// Evaluate a FEEL expression (`feel.eval_text`) against a dict of variables; the result
/// is a FEEL value as Python objects (`decimal.Decimal`, `str`, `bool`, `None`, `list`,
/// `datetime.date`, `datetime.timedelta`, `datetime.datetime`).
#[pyfunction]
#[pyo3(signature = (expr, context = None))]
fn feel_eval<'py>(
    py: Python<'py>,
    expr: &str,
    context: Option<&Bound<'py, PyDict>>,
) -> PyResult<Bound<'py, PyAny>> {
    let ctx = match context {
        Some(c) => context_from_py(c)?,
        None => Context::new(),
    };
    let v = fe::eval_text(expr, &ctx).map_err(|e| eval_error_to_py(py, e))?;
    value_to_py(py, &v)
}

// ------------------------------------------------------------------- simulation

/// A JSON tree (as `json.loads` would give it) as Python objects.
fn json_to_py<'py>(py: Python<'py>, j: &Json) -> PyResult<Bound<'py, PyAny>> {
    Ok(match j {
        Json::Null => py.None().into_bound(py),
        Json::Bool(b) => PyBool::new(py, *b).to_owned().into_any(),
        Json::Int(i) => i.into_pyobject(py)?.into_any(),
        Json::Num(t) => py
            .import("builtins")?
            .getattr("int")?
            .call1((t.as_str(),))?,
        Json::Float(f) => PyFloat::new(py, *f).into_any(),
        Json::Str(s) => PyString::new(py, s).into_any(),
        Json::Arr(items) => {
            let list = PyList::empty(py);
            for x in items {
                list.append(json_to_py(py, x)?)?;
            }
            list.into_any()
        }
        Json::Obj(pairs) => {
            let dict = PyDict::new(py);
            for (k, x) in pairs {
                dict.set_item(k, json_to_py(py, x)?)?;
            }
            dict.into_any()
        }
    })
}

/// `cases=` or `seed=` of `simulate`: the value Python's `float()` receives in `sim.py`.
fn param_of(obj: Option<&Bound<'_, PyAny>>) -> PyResult<Option<Param>> {
    let Some(obj) = obj else {
        return Ok(None);
    };
    if obj.is_none() {
        return Ok(None);
    }
    if obj.is_instance_of::<PyBool>() {
        let digits = if obj.extract::<bool>()? { "1" } else { "0" };
        return Ok(Some(Param::Int(digits.into())));
    }
    if obj.is_instance_of::<PyInt>() {
        return Ok(Some(Param::Int(obj.str()?.to_string())));
    }
    if obj.is_instance_of::<PyFloat>() {
        return Ok(Some(Param::Float(obj.extract::<f64>()?)));
    }
    if obj.is_instance_of::<PyString>() {
        return Ok(Some(Param::Text(obj.extract::<String>()?)));
    }
    Err(PyTypeError::new_err(format!(
        "float() argument must be a string or a real number, not '{}'",
        obj.get_type().name()?
    )))
}

/// The result of a simulation run, as `sim.Result`.
#[pyclass(name = "SimResult", module = "tok_engine", frozen)]
struct PySimResult {
    inner: SimResult,
}

#[pymethods]
impl PySimResult {
    /// The seed, `int(float(seed))`.
    #[getter]
    fn seed<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        json_to_py(py, &Json::Num(self.inner.outcome.seed.clone()))
    }

    /// Seconds from the first arrival to the last event.
    #[getter]
    fn horizon(&self) -> f64 {
        self.inner.outcome.horizon
    }

    /// Conditions and expressions that could not be evaluated, in order, with repetitions.
    #[getter]
    fn warnings(&self) -> Vec<String> {
        self.inner.outcome.warnings.clone()
    }

    /// The event log: one dict per activity instance (`Result.log`).
    #[getter]
    fn log<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let rows = self.inner.log().map_err(|e| eval_error_to_py(py, e))?;
        let list = PyList::empty(py);
        for r in &rows {
            list.append(json_to_py(py, &r.to_json())?)?;
        }
        Ok(list)
    }

    /// One dict per case with its variables (`Result.cases`).
    #[getter]
    fn cases<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let rows = self.inner.cases().map_err(|e| eval_error_to_py(py, e))?;
        let list = PyList::empty(py);
        for c in &rows {
            let dict = PyDict::new(py);
            dict.set_item("case", c.case)?;
            dict.set_item("start", &c.start)?;
            dict.set_item("end", &c.end)?;
            dict.set_item("end_event", &c.end_event)?;
            dict.set_item("cycle_time_s", c.cycle_time_s)?;
            let vars = PyDict::new(py);
            for (k, v) in &c.variables {
                vars.set_item(k, plain_to_py(py, v)?)?;
            }
            dict.set_item("variables", vars)?;
            list.append(dict)?;
        }
        Ok(list)
    }

    /// The indicators (`Result.kpi()`).
    fn kpi<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        json_to_py(py, &self.inner.kpi().to_json())
    }

    /// Indicators and cases (and the log) as JSON text (`Result.to_json`).
    #[pyo3(signature = (include_log = false))]
    fn to_json(&self, py: Python<'_>, include_log: bool) -> PyResult<String> {
        self.inner
            .to_json(include_log)
            .map_err(|e| eval_error_to_py(py, e))
    }

    /// Write the event log and the table of cases as CSV (`Result.write_csv`).
    #[pyo3(signature = (path_log, path_cases = None))]
    fn write_csv(
        &self,
        py: Python<'_>,
        path_log: Option<&str>,
        path_cases: Option<&str>,
    ) -> PyResult<()> {
        self.inner
            .write_csv(path_log, path_cases)
            .map_err(|e| eval_error_to_py(py, e))
    }

    /// The plain-text report of the indicators (`sim.format_kpi(result.kpi(), lang)`).
    #[pyo3(signature = (lang = "en"))]
    fn format_kpi(&self, lang: &str) -> String {
        self.inner.kpi().format(lang)
    }
}

/// Run the simulation of a document (`sim.simulate`) and return a `SimResult`.
/// `cases` and `seed` override the attributes of `simulate`; like `sim.py`, the simulator
/// takes `int(float(value))` of them. Raises `ValueError` for a document with errors and
/// the exceptions of the reference implementation (`tok_engine.SimError` for a process
/// without a start event, `ValueError`, `TypeError`, `OverflowError` …).
#[pyfunction]
#[pyo3(signature = (text, cases = None, seed = None))]
fn simulate(
    py: Python<'_>,
    text: &str,
    cases: Option<&Bound<'_, PyAny>>,
    seed: Option<&Bound<'_, PyAny>>,
) -> PyResult<PySimResult> {
    let doc = load_ok(text)?;
    let (cases, seed) = (param_of(cases)?, param_of(seed)?);
    let inner = tok_sim::simulate(&doc, cases, seed).map_err(|e| eval_error_to_py(py, e))?;
    Ok(PySimResult { inner })
}

/// `d[key]`.
fn item<'py>(d: &Bound<'py, PyAny>, key: &str) -> PyResult<Bound<'py, PyAny>> {
    d.get_item(key)
}

/// `d.get(key)`, `None` also for a stored `None`.
fn get<'py>(d: &Bound<'py, PyAny>, key: &str) -> PyResult<Option<Bound<'py, PyAny>>> {
    let v = d.call_method1("get", (key,))?;
    Ok(if v.is_none() { None } else { Some(v) })
}

/// `str(v)` of a value that may be missing (then the empty text).
fn text_or_empty(v: Option<Bound<'_, PyAny>>) -> PyResult<String> {
    match v {
        Some(v) => Ok(v.str()?.to_string()),
        None => Ok(String::new()),
    }
}

/// The plain-text report of a dict of indicators as `SimResult.kpi()` returns it
/// (`sim.format_kpi(kpi, lang)`).
#[pyfunction]
#[pyo3(signature = (kpi, lang = "en"))]
fn format_kpi(kpi: &Bound<'_, PyAny>, lang: &str) -> PyResult<String> {
    let cases = item(kpi, "cases")?;
    let mut by_end = Vec::new();
    for (k, v) in item(&cases, "by_end_event")?.cast::<PyDict>()?.iter() {
        by_end.push((k.str()?.to_string(), v.extract::<usize>()?));
    }
    let ct = item(kpi, "cycle_time")?;
    let mut activities = Vec::new();
    for (k, a) in item(kpi, "activities")?.cast::<PyDict>()?.iter() {
        activities.push((
            k.str()?.to_string(),
            ActivityKpi {
                label: text_or_empty(get(&a, "label")?)?,
                count: item(&a, "count")?.extract()?,
                mean_wait_s: 0.0,
                mean_processing_s: 0.0,
                mean_wait: item(&a, "mean_wait")?.str()?.to_string(),
                mean_processing: item(&a, "mean_processing")?.str()?.to_string(),
            },
        ));
    }
    let mut resources = Vec::new();
    if let Some(res) = get(kpi, "resources")? {
        for (k, r) in res.cast::<PyDict>()?.iter() {
            resources.push((
                k.str()?.to_string(),
                ResourceKpi {
                    label: text_or_empty(get(&r, "label")?)?,
                    count: item(&r, "count")?.extract::<f64>()?,
                    utilization: item(&r, "utilization")?.extract()?,
                },
            ));
        }
    }
    let warnings = match get(kpi, "warnings")? {
        Some(w) => w
            .try_iter()?
            .map(|x| Ok(x?.str()?.to_string()))
            .collect::<PyResult<_>>()?,
        None => Vec::new(),
    };
    let k = Kpi {
        seed: match get(kpi, "seed")? {
            Some(v) => v.str()?.to_string(),
            None => "None".into(),
        },
        started: item(&cases, "started")?.extract()?,
        finished: item(&cases, "finished")?.extract()?,
        by_end_event: by_end,
        mean_s: None,
        median_s: None,
        p90_s: None,
        mean: item(&ct, "mean")?.str()?.to_string(),
        median: item(&ct, "median")?.str()?.to_string(),
        p90: item(&ct, "p90")?.str()?.to_string(),
        activities,
        resources,
        horizon_s: 0.0,
        warnings,
    };
    Ok(k.format(lang))
}

#[pymodule]
fn tok_engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    m.add("FeelError", py.get_type::<FeelError>())?;
    m.add("FeelSyntaxError", py.get_type::<FeelSyntaxError>())?;
    m.add("FeelUnsupported", py.get_type::<FeelUnsupported>())?;
    m.add("DecisionError", py.get_type::<DecisionError>())?;
    m.add("SimError", py.get_type::<SimError>())?;
    m.add("DistError", py.get_type::<DistError>())?;
    m.add_class::<PySimResult>()?;
    m.add_function(wrap_pyfunction!(simulate, m)?)?;
    m.add_function(wrap_pyfunction!(format_kpi, m)?)?;
    m.add_function(wrap_pyfunction!(eval_decision, m)?)?;
    m.add_function(wrap_pyfunction!(feel_eval, m)?)?;
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

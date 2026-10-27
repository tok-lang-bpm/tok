//! Intermediate representation of a tok document. Mirrors `model.py`: elements refer to
//! each other by identifier, and every map keeps declaration order.

use indexmap::IndexMap;

pub const EVENT_KINDS: &[&str] = &["start", "end", "catch", "throw", "boundary"];
pub const TASK_KINDS: &[&str] = &[
    "task", "user", "service", "manual", "script", "rule", "send", "receive", "call",
];
pub const SUB_KINDS: &[&str] = &["sub", "eventsub"];
pub const GATEWAY_KINDS: &[&str] = &["xor", "and", "or", "eventgw"];

pub const TRIGGERS: &[&str] = &[
    "message",
    "timer",
    "signal",
    "error",
    "escalation",
    "condition",
    "link",
];
pub const TRIGGER_FLAGS: &[&str] = &["terminate", "compensate"];

pub const EVENT_KEYS: &[&str] = &[
    "on",
    "message",
    "timer",
    "signal",
    "error",
    "escalation",
    "code",
    "condition",
    "link",
    "correlation",
    "form",
    "delay",
    "assign",
];
pub const EVENT_FLAGS: &[&str] = &["noninterrupting", "terminate", "compensate"];
pub const TASK_KEYS: &[&str] = &[
    "message", "process", "form", "decision", "expr", "result", "type", "by", "duration", "assign",
];
pub const TASK_FLAGS: &[&str] = &["loop", "parallel", "sequential", "compensation"];

pub const HEADER_KEYS: &[&str] = &["id", "process", "target", "namespace", "collaboration"];
pub const HEADER_FLAGS: &[&str] = &["executable"];
pub const POOL_KEYS: &[&str] = &["process"];
pub const POOL_FLAGS: &[&str] = &["executable"];
pub const FLOW_KEYS: &[&str] = &["id", "when", "p"];
pub const FLOW_FLAGS: &[&str] = &["default"];
pub const MESSAGE_KEYS: &[&str] = &["id", "message", "delay"];
pub const DATA_KEYS: &[&str] = &["state"];
pub const DATA_FLAGS: &[&str] = &["collection"];
pub const NOTE_KEYS: &[&str] = &["on"];
pub const GROUP_KEYS: &[&str] = &["members"];
pub const DECISION_KEYS: &[&str] = &["hit"];
pub const FIELD_KEYS: &[&str] = &["type", "options", "min", "max", "default"];
pub const FIELD_FLAGS: &[&str] = &["required"];
pub const SIMULATE_KEYS: &[&str] = &["cases", "seed", "arrival", "start"];
pub const RESOURCE_KEYS: &[&str] = &["count"];

pub const HIT_POLICIES: &[&str] = &["unique", "first", "any", "priority", "collect"];
pub const FIELD_TYPES: &[&str] = &[
    "textfield",
    "textarea",
    "number",
    "checkbox",
    "select",
    "radio",
    "datetime",
];

pub fn is_event(kind: &str) -> bool {
    EVENT_KINDS.contains(&kind)
}
pub fn is_sub(kind: &str) -> bool {
    SUB_KINDS.contains(&kind)
}
pub fn is_node(kind: &str) -> bool {
    EVENT_KINDS.contains(&kind)
        || TASK_KINDS.contains(&kind)
        || is_sub(kind)
        || GATEWAY_KINDS.contains(&kind)
}
pub fn is_activity(kind: &str) -> bool {
    TASK_KINDS.contains(&kind) || is_sub(kind)
}

/// Canonical attribute keys of a flow node kind.
pub fn node_keys(kind: &str) -> &'static [&'static str] {
    if is_event(kind) {
        EVENT_KEYS
    } else if TASK_KINDS.contains(&kind) || is_sub(kind) {
        TASK_KEYS
    } else {
        &[]
    }
}

/// Canonical flags of a flow node kind.
pub fn node_flags(kind: &str) -> &'static [&'static str] {
    if is_event(kind) {
        EVENT_FLAGS
    } else if TASK_KINDS.contains(&kind) || is_sub(kind) {
        TASK_FLAGS
    } else {
        &[]
    }
}

/// Attribute map in insertion order.
pub type Attrs = IndexMap<String, String>;

/// Fields shared by every statement.
#[derive(Debug, Clone, Default)]
pub struct Base {
    pub kind: String,
    pub id: Option<String>,
    pub label: Option<String>,
    pub attrs: Attrs,
    pub flags: Vec<String>,
    pub line: usize,
    pub comments: Vec<String>,
    pub trailing: Option<String>,
}

impl Base {
    pub fn new(
        kind: &str,
        id: Option<String>,
        label: Option<String>,
        attrs: Attrs,
        flags: Vec<String>,
        line: usize,
    ) -> Self {
        Base {
            kind: kind.to_string(),
            id,
            label,
            attrs,
            flags,
            line,
            comments: Vec::new(),
            trailing: None,
        }
    }
    pub fn has(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f == flag)
    }
    pub fn id(&self) -> &str {
        self.id.as_deref().unwrap_or("")
    }
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.get(key).map(String::as_str)
    }
}

/// A flow node.
#[derive(Debug, Clone, Default)]
pub struct Node {
    pub base: Base,
    pub scope: Option<String>,
    pub lane: Option<String>,
    pub pool: Option<String>,
    pub children: Vec<String>,
}

/// A sequence flow.
#[derive(Debug, Clone, Default)]
pub struct Flow {
    pub base: Base,
    pub source: String,
    pub target: String,
    pub explicit_id: bool,
}

/// A message flow.
#[derive(Debug, Clone, Default)]
pub struct MessageFlow {
    pub base: Base,
    pub source: String,
    pub target: String,
    pub explicit_id: bool,
}

/// A data object (`data`) or data store (`store`).
#[derive(Debug, Clone, Default)]
pub struct Data {
    pub base: Base,
    pub scope: Option<String>,
    pub block: Option<String>,
}

/// `reads` or `writes`.
#[derive(Debug, Clone, Default)]
pub struct Association {
    pub base: Base,
    pub activity: String,
    pub data: String,
}

/// A pool.
#[derive(Debug, Clone, Default)]
pub struct Pool {
    pub base: Base,
    pub blackbox: bool,
    pub children: Vec<String>,
}

impl Pool {
    pub fn process_id(&self) -> String {
        match self.base.attrs.get("process") {
            Some(p) if !p.is_empty() => p.clone(),
            _ => format!("{}-process", self.base.id()),
        }
    }
}

/// A lane.
#[derive(Debug, Clone, Default)]
pub struct Lane {
    pub base: Base,
    pub pool: Option<String>,
    pub parent: Option<String>,
    pub children: Vec<String>,
    /// Child lanes in XML order (BPMN import only).
    pub sub: Vec<String>,
}

/// One rule of a decision table.
#[derive(Debug, Clone, Default)]
pub struct Rule {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub annotation: Option<String>,
    pub line: usize,
    pub comments: Vec<String>,
    pub trailing: Option<String>,
}

/// A decision table.
#[derive(Debug, Clone, Default)]
pub struct Decision {
    pub base: Base,
    pub inputs: Vec<(String, String)>,
    pub outputs: Vec<(String, String)>,
    pub rules: Vec<Rule>,
    pub has_annotations: bool,
    pub annotation_title: Option<String>,
    pub header_line: Option<usize>,
    pub header_done: bool,
}

impl Decision {
    pub fn hit(&self) -> &str {
        self.base
            .attrs
            .get("hit")
            .map(String::as_str)
            .unwrap_or("unique")
    }
}

/// A form item: `text` or `field`.
#[derive(Debug, Clone, Default)]
pub struct FormItem {
    pub base: Base,
}

/// A form.
#[derive(Debug, Clone, Default)]
pub struct Form {
    pub base: Base,
    pub items: Vec<FormItem>,
}

/// Kind and line of a registered identifier (for duplicate and reference checks).
#[derive(Debug, Clone)]
pub struct IdEntry {
    pub kind: String,
    pub line: usize,
}

/// A parsed document.
#[derive(Debug, Clone, Default)]
pub struct Document {
    pub version: Option<String>,
    pub title: Option<String>,
    pub attrs: Attrs,
    pub flags: Vec<String>,
    pub header_line: usize,
    pub header_comments: Vec<String>,
    pub header_trailing: Option<String>,
    pub top: Vec<String>,
    pub nodes: IndexMap<String, Node>,
    pub flows: Vec<Flow>,
    pub messages: Vec<MessageFlow>,
    pub data: IndexMap<String, Data>,
    pub assocs: Vec<Association>,
    pub notes: Vec<Base>,
    pub groups: Vec<Base>,
    pub pools: Vec<Pool>,
    pub lanes: IndexMap<String, Lane>,
    pub decisions: IndexMap<String, Decision>,
    pub forms: IndexMap<String, Form>,
    pub simulate: Option<Base>,
    pub resources: IndexMap<String, Base>,
    pub variables: Vec<Base>,
    pub end_comments: Vec<String>,
    pub ids: IndexMap<String, IdEntry>,
}

impl Document {
    pub fn executable(&self) -> bool {
        self.flags.iter().any(|f| f == "executable")
    }
    pub fn executable_any(&self) -> bool {
        self.executable() || self.pools.iter().any(|p| p.base.has("executable"))
    }
    pub fn default_process_id(&self) -> String {
        self.attrs
            .get("process")
            .cloned()
            .unwrap_or_else(|| "process".to_string())
    }
    pub fn pool(&self, id: &str) -> Option<&Pool> {
        self.pools.iter().find(|p| p.base.id() == id)
    }
    pub fn pool_mut(&mut self, id: &str) -> Option<&mut Pool> {
        self.pools.iter_mut().find(|p| p.base.id() == id)
    }
}

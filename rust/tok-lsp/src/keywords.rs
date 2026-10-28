//! The keywords of the language with a short description for hover and completion, the
//! symbol kinds of the outline, and the attribute keys offered after a statement's
//! identifier.

use tok_core::model::{
    CHOREO_FLAGS, CHOREO_KEYS, COMPLEX_KEYS, DATA_FLAGS, DATA_KEYS, EVENT_FLAGS, EVENT_KEYS,
    EVENTGW_FLAGS, EXEC_FLAGS, EXEC_KEYS, FIELD_FLAGS, FIELD_KEYS, FLOW_FLAGS, FLOW_KEYS,
    GATEWAY_KEYS, GLOBAL_KEYS, GROUP_KEYS, IO_FLAGS, IO_KEYS, LANE_KEYS, MESSAGE_KEYS, NOTE_KEYS,
    OPERATION_KEYS, POOL_FLAGS, POOL_KEYS, POOL_KEYS_06, PROP_KEYS, RESOURCE_KEYS, SIMULATE_KEYS,
    TASK_FLAGS, TASK_KEYS, USER_TASK_KEYS,
};

/// Keyword, description (with the element it exports and the section of the specification).
pub const KEYWORDS: &[(&str, &str)] = &[
    (
        "tok",
        "document header: language version, title and document attributes (§4)",
    ),
    ("start", "start event (BPMN `startEvent`, §5.1)"),
    ("end", "end event (BPMN `endEvent`, §5.1)"),
    (
        "catch",
        "intermediate catch event; needs a trigger (BPMN `intermediateCatchEvent`, §5.1)",
    ),
    (
        "throw",
        "intermediate throw event (BPMN `intermediateThrowEvent`, §5.1)",
    ),
    (
        "boundary",
        "boundary event of an activity, `on=ACTIVITY` (BPMN `boundaryEvent`, §5.1)",
    ),
    ("task", "task (BPMN `task`, §5.2)"),
    ("user", "user task (BPMN `userTask`, §5.2)"),
    ("service", "service task (BPMN `serviceTask`, §5.2)"),
    ("manual", "manual task (BPMN `manualTask`, §5.2)"),
    (
        "script",
        "script task, `expr=` and `result=` (BPMN `scriptTask`, §5.2)",
    ),
    (
        "rule",
        "business rule task calling `decision=` (BPMN `businessRuleTask`, §5.2)",
    ),
    ("send", "send task (BPMN `sendTask`, §5.2)"),
    ("receive", "receive task (BPMN `receiveTask`, §5.2)"),
    (
        "call",
        "call activity of `process=` or `task=` (BPMN `callActivity`, §5.2)",
    ),
    (
        "sub",
        "embedded sub-process with its own block (BPMN `subProcess`, §5.3)",
    ),
    (
        "eventsub",
        "event sub-process (BPMN `subProcess triggeredByEvent`, §5.3)",
    ),
    ("adhoc", "ad-hoc sub-process (BPMN `adHocSubProcess`, §5.3)"),
    (
        "transaction",
        "transaction (BPMN `transaction`, language 0.5, §5.3)",
    ),
    ("xor", "exclusive gateway (BPMN `exclusiveGateway`, §5.4)"),
    ("and", "parallel gateway (BPMN `parallelGateway`, §5.4)"),
    ("or", "inclusive gateway (BPMN `inclusiveGateway`, §5.4)"),
    (
        "eventgw",
        "event-based gateway (BPMN `eventBasedGateway`, §5.4)",
    ),
    (
        "complex",
        "complex gateway (BPMN `complexGateway`, language 0.5, §5.4)",
    ),
    (
        "pool",
        "pool: a participant with its process, or a black box without a block (§5.5)",
    ),
    (
        "lane",
        "lane of a pool; its block declares the flow nodes of the lane (§5.5)",
    ),
    (
        "global",
        "global task (BPMN `globalTask`, language 0.5, §5.6)",
    ),
    (
        "interface",
        "interface with a block of operations (BPMN `interface`, language 0.5, §5.6)",
    ),
    (
        "operation",
        "operation of an interface, `in=\"Message\"` (BPMN `operation`, §5.6)",
    ),
    (
        "choreography",
        "choreography with its own block (BPMN `choreography`, language 0.6, §5.7)",
    ),
    (
        "choreo",
        "choreography task, `from=` and `to=` pools (BPMN `choreographyTask`, §5.7)",
    ),
    (
        "subchoreo",
        "sub-choreography (BPMN `subChoreography`, language 0.6, §5.7)",
    ),
    (
        "callchoreo",
        "call choreography (BPMN `callChoreography`, language 0.6, §5.7)",
    ),
    (
        "flow",
        "sequence flows: `flow A -> B -> C`, or one flow with a label and attributes (§6.1)",
    ),
    (
        "message",
        "message flow between pools: `message A -> B \"Label\"` (§6.2)",
    ),
    (
        "key",
        "correlation key with `prop` lines (BPMN `correlationKey`, language 0.5, §6.3)",
    ),
    (
        "prop",
        "correlation property of a key, `message=` and `path=` (§6.3)",
    ),
    (
        "conversation",
        "conversation between pools (BPMN `conversation`, language 0.6, §6.4)",
    ),
    (
        "subconversation",
        "sub-conversation (BPMN `subConversation`, language 0.6, §6.4)",
    ),
    (
        "callconversation",
        "call conversation (BPMN `callConversation`, language 0.6, §6.4)",
    ),
    (
        "data",
        "data object reference (BPMN `dataObjectReference`, §7)",
    ),
    (
        "store",
        "data store reference (BPMN `dataStoreReference`, §7)",
    ),
    (
        "reads",
        "data input association: `reads ACTIVITY DATA` (§7)",
    ),
    (
        "writes",
        "data output association: `writes ACTIVITY DATA` (§7)",
    ),
    (
        "item",
        "item definition, or in language 0.7 a DMN type (§7.1, §9.4)",
    ),
    (
        "input",
        "data input of the process or of an activity (BPMN `dataInput`, §7.1)",
    ),
    (
        "output",
        "data output of the process or of an activity (BPMN `dataOutput`, §7.1)",
    ),
    (
        "property",
        "property of a process, activity or event (BPMN `property`, §7.1)",
    ),
    (
        "resource",
        "resource: performers in simulation and, from 0.5, a BPMN resource (§7.2, §11)",
    ),
    ("note", "text annotation (BPMN `textAnnotation`, §8)"),
    (
        "group",
        "group of elements, `members=\"…\"` (BPMN `group`, §8)",
    ),
    (
        "decision",
        "DMN decision: a decision table, or in language 0.7 any decision logic (§9)",
    ),
    (
        "inputdata",
        "DMN input data (`inputData`, language 0.7, §9.1)",
    ),
    (
        "source",
        "DMN knowledge source (`knowledgeSource`, language 0.7, §9.1)",
    ),
    (
        "bkm",
        "DMN business knowledge model, a function (`businessKnowledgeModel`, language 0.7, §9.1)",
    ),
    (
        "entry",
        "context entry of decision logic (language 0.7, §9.2)",
    ),
    ("result", "result of a context (language 0.7, §9.2)"),
    ("element", "element of a list (language 0.7, §9.2)"),
    ("form", "Camunda form with `text` and `field` lines (§10.2)"),
    (
        "text",
        "text of a form, Markdown with `{{variable}}` (§10.2)",
    ),
    (
        "field",
        "field of a form; its key is a process variable (§10.2)",
    ),
    (
        "simulate",
        "simulation settings: `cases=`, `seed=`, `arrival=`, `start=` (§11)",
    ),
    (
        "variable",
        "case variable drawn at the start of each simulated case (§11)",
    ),
];

/// The description of a keyword.
pub fn describe(word: &str) -> Option<&'static str> {
    KEYWORDS.iter().find(|(k, _)| *k == word).map(|(_, d)| *d)
}

/// The LSP symbol kind of a statement in the outline, or `None` for statements that are
/// not shown.
pub fn symbol_kind(keyword: &str) -> Option<u32> {
    const MODULE: u32 = 2;
    const NAMESPACE: u32 = 3;
    const CLASS: u32 = 5;
    const METHOD: u32 = 6;
    const PROPERTY: u32 = 7;
    const FIELD: u32 = 8;
    const INTERFACE: u32 = 11;
    const FUNCTION: u32 = 12;
    const VARIABLE: u32 = 13;
    const STRING: u32 = 15;
    const ARRAY: u32 = 18;
    const OBJECT: u32 = 19;
    const KEY: u32 = 20;
    const STRUCT: u32 = 23;
    const EVENT: u32 = 24;
    const OPERATOR: u32 = 25;
    const TYPE_PARAMETER: u32 = 26;
    Some(match keyword {
        "pool" | "choreography" => MODULE,
        "lane" | "conversation" | "subconversation" | "callconversation" => NAMESPACE,
        "start" | "end" | "catch" | "throw" | "boundary" => EVENT,
        "task" | "user" | "service" | "manual" | "script" | "rule" | "send" | "receive"
        | "call" | "choreo" | "callchoreo" | "operation" => METHOD,
        "sub" | "eventsub" | "adhoc" | "transaction" | "subchoreo" => CLASS,
        "xor" | "and" | "or" | "eventgw" | "complex" => OPERATOR,
        "data" | "store" | "input" | "output" | "property" | "inputdata" | "variable" => VARIABLE,
        "decision" | "bkm" => FUNCTION,
        "entry" | "result" | "prop" => PROPERTY,
        "form" => STRUCT,
        "field" => FIELD,
        "item" => TYPE_PARAMETER,
        "global" | "interface" => INTERFACE,
        "resource" | "source" => OBJECT,
        "key" => KEY,
        "note" => STRING,
        "group" => ARRAY,
        _ => return None,
    })
}

/// The attribute keys and flags of a statement, for completion after its identifier.
pub fn attributes(keyword: &str) -> Vec<&'static str> {
    let groups: &[&[&str]] = match keyword {
        "start" | "end" | "catch" | "throw" | "boundary" => &[EVENT_KEYS, EVENT_FLAGS],
        "user" => &[TASK_KEYS, TASK_FLAGS, EXEC_KEYS, USER_TASK_KEYS],
        "task" | "service" | "manual" | "script" | "rule" | "send" | "receive" | "call" => {
            &[TASK_KEYS, TASK_FLAGS, EXEC_KEYS]
        }
        "sub" | "eventsub" | "adhoc" | "transaction" => {
            &[TASK_KEYS, TASK_FLAGS, EXEC_KEYS, EXEC_FLAGS]
        }
        "complex" => &[COMPLEX_KEYS],
        "eventgw" => &[GATEWAY_KEYS, EVENTGW_FLAGS],
        "xor" | "and" | "or" => &[GATEWAY_KEYS],
        "pool" => &[POOL_KEYS, POOL_KEYS_06, POOL_FLAGS],
        "lane" => &[LANE_KEYS],
        "flow" => &[FLOW_KEYS, FLOW_FLAGS],
        "message" => &[MESSAGE_KEYS],
        "data" | "store" => &[DATA_KEYS, DATA_FLAGS],
        "input" | "output" | "property" => &[IO_KEYS, IO_FLAGS],
        "note" => &[NOTE_KEYS],
        "group" => &[GROUP_KEYS],
        "field" => &[FIELD_KEYS, FIELD_FLAGS],
        "simulate" => &[SIMULATE_KEYS],
        "resource" => &[RESOURCE_KEYS],
        "global" => &[GLOBAL_KEYS],
        "operation" => &[OPERATION_KEYS],
        "prop" => &[PROP_KEYS],
        "choreo" => &[CHOREO_KEYS, CHOREO_FLAGS],
        _ => &[],
    };
    let mut out: Vec<&'static str> = Vec::new();
    for g in groups {
        for k in g.iter() {
            if !out.contains(k) {
                out.push(k);
            }
        }
    }
    out
}

/// Whether `word` is a flag (written without `=`) for `keyword`.
pub fn is_flag(keyword: &str, word: &str) -> bool {
    let flags: &[&[&str]] = match keyword {
        "start" | "end" | "catch" | "throw" | "boundary" => &[EVENT_FLAGS],
        "task" | "user" | "service" | "manual" | "script" | "rule" | "send" | "receive"
        | "call" | "sub" | "eventsub" | "adhoc" | "transaction" => &[TASK_FLAGS, EXEC_FLAGS],
        "eventgw" => &[EVENTGW_FLAGS],
        "pool" => &[POOL_FLAGS],
        "flow" => &[FLOW_FLAGS],
        "data" | "store" => &[DATA_FLAGS],
        "input" | "output" | "property" => &[IO_FLAGS],
        "field" => &[FIELD_FLAGS],
        "choreo" => &[CHOREO_FLAGS],
        _ => &[],
    };
    flags.iter().any(|g| g.contains(&word))
}

/// The layer of a diagnostic code (spec §13).
pub fn layer(code: &str) -> &'static str {
    match code.chars().next() {
        Some('T') => "tokens and lines",
        Some('P') => "statements",
        Some('R') => "references and structure of the document",
        Some('E') => "BPMN 2.0 rules: error",
        Some('W') => "BPMN 2.0 rules: warning",
        Some('L') => "BPMN 2.0 rules: style note",
        Some('C') => "executable profile for Camunda 8",
        Some('D') => "decision tables and relations",
        Some('F') => "FEEL",
        Some('S') => "simulation parameters",
        Some('M') => "forms",
        _ => "diagnostics",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The TextMate grammar of the VS Code extension highlights the same keywords.
    #[test]
    fn the_grammar_knows_every_keyword() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../editors/vscode/syntaxes/tok.tmLanguage.json");
        let text = std::fs::read_to_string(path).expect("the grammar exists");
        let grammar: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        let pattern = grammar["repository"]["declaration"]["match"]
            .as_str()
            .expect("a match pattern");
        let start = pattern.find('(').expect("a group") + 1;
        let end = start + pattern[start..].find(')').expect("its end");
        let mut words: Vec<&str> = pattern[start..end].split('|').collect();
        words.extend(["tok", "flow", "message", "reads", "writes"]);
        words.sort_unstable();
        let mut known: Vec<&str> = KEYWORDS.iter().map(|(k, _)| *k).collect();
        known.sort_unstable();
        assert_eq!(words, known);
    }
}

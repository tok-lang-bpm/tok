//! Generators of large valid tok documents, for the benchmarks of the engine
//! (`benches/engine.rs`, see `docs/performance.md`) and for tests that need big models.
//!
//! Every generator is deterministic: the same arguments give the same text.

use std::fmt::Write;

/// A process of about `n` flow nodes without pools: blocks of a user task, an exclusive
/// split with two branches and a join; a timer boundary event on every seventh task, which
/// leads to a notification and an end event of its own; a data object read and written
/// every tenth block; the simulation parameters of every activity.
pub fn process(n: usize) -> String {
    let blocks = (n.saturating_sub(2) / 5).max(1);
    let mut t = String::new();
    let _ = writeln!(
        t,
        "tok 0.5 \"Generated process of {n} elements\"
"
    );
    let _ = writeln!(t, "start s \"Start\"");
    for i in 1..=blocks {
        let _ = writeln!(
            t,
            "user t{i} \"Task {i}\" by=staff duration=\"tri(5m, 10m, 30m)\""
        );
        if i % 7 == 0 {
            let _ = writeln!(
                t,
                "boundary bt{i} \"Too long {i}\" on=t{i} timer=PT1H noninterrupting"
            );
        }
        let _ = writeln!(t, "xor g{i} \"Accepted {i}?\"");
        let _ = writeln!(t, "task a{i} \"Accept {i}\" duration=\"5m\"");
        let _ = writeln!(t, "task b{i} \"Reject {i}\" duration=\"exp(15m)\"");
        let _ = writeln!(t, "xor j{i}");
        if i % 10 == 0 {
            let _ = writeln!(t, "data d{i} \"Record {i}\" state=\"checked\"");
        }
    }
    let _ = writeln!(t, "end e \"End\"");
    for i in (7..=blocks).step_by(7) {
        let _ = writeln!(t, "task n{i} \"Notify {i}\"");
        let _ = writeln!(t, "end en{i} \"Notified {i}\"");
    }
    t.push('\n');
    let _ = writeln!(t, "flow s -> t1");
    for i in 1..=blocks {
        let _ = writeln!(t, "flow t{i} -> g{i}");
        let _ = writeln!(t, "flow g{i} -> a{i} \"yes\" when=\"x > 0.3\"");
        let _ = writeln!(t, "flow g{i} -> b{i} \"no\" default");
        let _ = writeln!(t, "flow a{i} -> j{i}");
        let _ = writeln!(t, "flow b{i} -> j{i}");
        if i == blocks {
            let _ = writeln!(t, "flow j{i} -> e");
        } else {
            let _ = writeln!(t, "flow j{i} -> t{}", i + 1);
        }
    }
    for i in (7..=blocks).step_by(7) {
        let _ = writeln!(t, "flow bt{i} -> n{i} -> en{i}");
    }
    for i in (10..=blocks).step_by(10) {
        let _ = writeln!(t, "reads t{i} d{i}");
        let _ = writeln!(t, "writes a{i} d{i}");
    }
    t.push('\n');
    let _ = writeln!(t, "simulate cases=100 seed=1 arrival=\"exp(20m)\"");
    let _ = writeln!(t, "resource staff \"Staff\" count=8");
    let _ = writeln!(t, "variable x \"unif(0, 1)\"");
    t
}

/// A collaboration of `pools` pools with `lanes` lanes each and `per_lane` tasks in every
/// lane; the tasks of a pool form one chain through its lanes, and each pool sends a
/// message to the next.
pub fn collaboration(pools: usize, lanes: usize, per_lane: usize) -> String {
    let mut t = String::new();
    let _ = writeln!(
        t,
        "tok 0.5 \"Generated collaboration of {pools} pools and {lanes} lanes\"\n"
    );
    let mut flows = String::new();
    let mut messages = String::new();
    for p in 1..=pools {
        let _ = writeln!(t, "pool p{p} \"Organisation {p}\"");
        let mut chain = vec![format!("p{p}s")];
        for l in 1..=lanes {
            let _ = writeln!(t, "  lane p{p}l{l} \"Role {p}.{l}\"");
            if l == 1 {
                let _ = writeln!(t, "    start p{p}s \"Request {p}\"");
            }
            for k in 1..=per_lane {
                let _ = writeln!(t, "    task p{p}l{l}t{k} \"Work {p}.{l}.{k}\"");
                chain.push(format!("p{p}l{l}t{k}"));
            }
            if l == lanes {
                let _ = writeln!(t, "    end p{p}e \"Done {p}\"");
            }
        }
        chain.push(format!("p{p}e"));
        for pair in chain.windows(2) {
            let _ = writeln!(flows, "flow {} -> {}", pair[0], pair[1]);
        }
        if p < pools {
            let _ = writeln!(
                messages,
                "message p{p}l{lanes}t{per_lane} -> p{}l1t1 \"Message {p}\"",
                p + 1
            );
        }
        t.push('\n');
    }
    t.push_str(&flows);
    t.push('\n');
    t.push_str(&messages);
    t
}

/// Sub-processes nested `depth` levels deep, each with a start event, a task before and
/// after the inner sub-process, and an end event.
pub fn nested(depth: usize) -> String {
    fn level(t: &mut String, i: usize, depth: usize) {
        let ind = "  ".repeat(i + 1);
        let _ = writeln!(t, "{ind}start s{i}");
        let _ = writeln!(t, "{ind}task a{i} \"Before {i}\"");
        if i + 1 < depth {
            let _ = writeln!(t, "{ind}sub p{} \"Level {}\"", i + 1, i + 1);
            level(t, i + 1, depth);
        }
        let _ = writeln!(t, "{ind}task b{i} \"After {i}\"");
        let _ = writeln!(t, "{ind}end e{i}");
        if i + 1 < depth {
            let _ = writeln!(t, "{ind}flow s{i} -> a{i} -> p{} -> b{i} -> e{i}", i + 1);
        } else {
            let _ = writeln!(t, "{ind}flow s{i} -> a{i} -> b{i} -> e{i}");
        }
    }
    let mut t = String::new();
    let _ = writeln!(t, "tok 0.5 \"Sub-processes nested {depth} levels deep\"\n");
    let _ = writeln!(t, "start s");
    let _ = writeln!(t, "sub p0 \"Level 0\"");
    level(&mut t, 0, depth.max(1));
    let _ = writeln!(t, "end e");
    let _ = writeln!(t, "flow s -> p0 -> e");
    t
}

/// A decision table of language 0.7 with `rows` rules over a number and a string input
/// (`hit=first`), called by a rule task.
pub fn decision_table(rows: usize) -> String {
    let mut t = String::new();
    let _ = writeln!(t, "tok 0.7 \"Decision table of {rows} rules\"\n");
    let _ = writeln!(t, "start s");
    let _ = writeln!(t, "rule r \"Rate\" decision=rate result=rate");
    let _ = writeln!(t, "end e");
    let _ = writeln!(t, "flow s -> r -> e\n");
    let _ = writeln!(t, "inputdata amount \"Amount\" type=number");
    let _ = writeln!(t, "inputdata kind \"Kind\" type=string");
    let _ = writeln!(
        t,
        "decision rate \"Rate\" requires=\"amount kind\" hit=first"
    );
    let _ = writeln!(t, "  | amount: number | kind: string || rate: number |");
    for i in 0..rows {
        let _ = writeln!(
            t,
            "  | [{}..{}) | \"k{}\" || {} |",
            i * 10,
            i * 10 + 10,
            i % 7,
            i
        );
    }
    let _ = writeln!(t, "  | - | - || -1 |");
    t
}

/// A decision requirements graph of `n` decisions, each requiring the one before it, with
/// literal expressions, and a business knowledge model they all call.
pub fn graph(n: usize) -> String {
    let mut t = String::new();
    let _ = writeln!(t, "tok 0.7 \"Decision graph of {n} decisions\"\n");
    let _ = writeln!(t, "inputdata x \"Input\" type=number");
    let _ = writeln!(
        t,
        "bkm step \"Step\" params=\"a: number, b: number\" type=number expr=\"a + b * 2\""
    );
    let _ = writeln!(
        t,
        "decision d0 \"Decision 0\" type=number requires=\"x step\" expr=\"step(x, 1)\""
    );
    for i in 1..n {
        let _ = writeln!(
            t,
            "decision d{i} \"Decision {i}\" type=number requires=\"d{} step\" expr=\"step(d{}, {i}) - d{} * 2\"",
            i - 1,
            i - 1,
            i - 1
        );
    }
    t
}

/// A long FEEL expression of about `terms` terms that stays within the nesting the parser
/// reads: a sum over a list of conditional terms, filtered and iterated.
pub fn feel(terms: usize) -> String {
    let mut items = Vec::with_capacity(terms);
    for i in 0..terms {
        items.push(format!(
            "if x > {i} then {i} * 2 + {} else string length(\"k{i}\")",
            i % 13
        ));
    }
    format!(
        "sum(for v in [{}][item >= 0] return v + count([1, 2, 3]))",
        items.join(", ")
    )
}

/// A document of language 0.7 whose decision is the expression of [`feel`].
pub fn feel_document(terms: usize) -> String {
    format!(
        "tok 0.7 \"Long FEEL of {terms} terms\"\n\ninputdata x \"Input\" type=number\ndecision long \"Long\" type=number requires=\"x\" expr=\"{}\"\n",
        feel(terms).replace('\\', "\\\\").replace('"', "\\\"")
    )
}

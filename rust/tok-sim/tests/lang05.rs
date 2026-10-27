//! Simulation of the elements of language 0.5: transactions with cancel end events,
//! complex gateways, standard loops with a condition, multiple and parallel multiple
//! events, and instantiating receive tasks and event-based gateways.

use std::path::{Path, PathBuf};

use tok_sim::sim::{Case, Row};
use tok_sim::{SimResult, simulate};

fn run(text: &str) -> SimResult {
    let (doc, diags) = tok_core::load(text, "full", None);
    let errors: Vec<String> = diags
        .iter()
        .filter(|d| d.is_error())
        .map(|d| format!("{d:?}"))
        .collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    simulate(&doc, None, None).unwrap_or_else(|e| panic!("{e}"))
}

fn rows(r: &SimResult) -> &[Row] {
    &r.outcome.rows
}

fn cases(r: &SimResult) -> &[Case] {
    &r.outcome.cases
}

/// Activities of the event log of case `case`, in log order.
fn trace(r: &SimResult, case: u64) -> Vec<String> {
    rows(r)
        .iter()
        .filter(|x| x.case == case)
        .map(|x| x.activity.clone())
        .collect()
}

fn row<'a>(r: &'a SimResult, case: u64, activity: &str) -> &'a Row {
    rows(r)
        .iter()
        .find(|x| x.case == case && x.activity == activity)
        .unwrap_or_else(|| panic!("no row of '{activity}' in case {case}"))
}

fn count(r: &SimResult, activity: &str) -> usize {
    rows(r).iter().filter(|x| x.activity == activity).count()
}

const H: f64 = 3600.0;

// ---------------------------------------------------------------- transactions

const TRANSACTION: &str = r#"tok 0.5 "Transaction"

start s
transaction tx "Booking"
  start ts
  task room "Book a room" compensate=free-room duration="1h"
  task free-room "Free the room" compensation duration="10m"
  task ticket "Buy a ticket" compensate=refund duration="1h"
  task refund "Refund the ticket" compensation duration="5m"
  xor ok "Booked?"
  end booked "Booked"
  end abort "Abort" cancel
  flow ts -> room -> ticket -> ok
  flow ok -> booked "yes" when="success"
  flow ok -> abort "no" default
boundary cancelled "Booking cancelled" on=tx cancel
task confirm "Confirm"
task inform "Inform the customer"
end e-ok "Trip booked"
end e-cancel "Trip cancelled"

flow s -> tx -> confirm -> e-ok
flow cancelled -> inform -> e-cancel

simulate cases=2 seed=1 arrival="fixed(1d)"
variable success "fixed(0)"
"#;

#[test]
fn cancel_end_event_compensates_and_leaves_through_the_cancel_boundary() {
    let r = run(TRANSACTION);
    for c in cases(&r) {
        assert_eq!(c.end_event.as_deref(), Some("e-cancel"));
    }
    // the handlers start together, the latest completed activity first, and finish
    // before the transaction is left
    assert_eq!(
        trace(&r, 1),
        ["room", "ticket", "refund", "free-room", "tx", "inform"]
    );
    assert_eq!(row(&r, 1, "refund").enqueue, 2.0 * H);
    assert_eq!(row(&r, 1, "free-room").enqueue, 2.0 * H);
    let tx = row(&r, 1, "tx");
    assert_eq!(tx.status, "interrupted");
    assert_eq!(tx.end, Some(2.0 * H + 600.0));
    assert_eq!(row(&r, 1, "inform").enqueue, 2.0 * H + 600.0);
    assert_eq!(count(&r, "confirm"), 0);
    assert_eq!(count(&r, "refund"), 2);
}

#[test]
fn transaction_that_ends_normally_runs_like_a_sub_process() {
    let text = TRANSACTION.replace("\"fixed(0)\"", "\"fixed(1)\"");
    let r = run(&text);
    assert_eq!(trace(&r, 1), ["room", "ticket", "tx", "confirm"]);
    assert_eq!(row(&r, 1, "tx").status, "completed");
    for c in cases(&r) {
        assert_eq!(c.end_event.as_deref(), Some("e-ok"));
    }
}

#[test]
fn cancel_without_cancel_boundary_ends_the_path() {
    let text = TRANSACTION
        .replace(
            "boundary cancelled \"Booking cancelled\" on=tx cancel\n",
            "",
        )
        .replace(
            "flow cancelled -> inform -> e-cancel\n",
            "flow inform -> e-cancel\n",
        );
    let (doc, _) = tok_core::load(&text, "full", None);
    let r = simulate(&doc, None, None).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        trace(&r, 1),
        ["room", "ticket", "refund", "free-room", "tx"]
    );
    assert_eq!(count(&r, "confirm"), 0);
    let c = &cases(&r)[0];
    assert_eq!(c.end, Some(2.0 * H + 600.0));
}

// ------------------------------------------------------------ complex gateway

const COMPLEX: &str = r#"tok 0.5 "Complex"

start s
and split
task r1 "Review 1" duration="1h"
task r2 "Review 2" duration="2h"
task r3 "Review 3" duration="3h"
complex enough "Enough reviews?" activation="_activationCount >= 2"
task decide "Decide" duration="30m"
end e "Decided"

flow s -> split
flow split -> r1
flow split -> r2
flow split -> r3
flow r1 -> enough
flow r2 -> enough
flow r3 -> enough
flow enough -> decide -> e

simulate cases=3 seed=1 arrival="fixed(1d)"
"#;

#[test]
fn complex_join_fires_once_when_activation_holds() {
    let r = run(COMPLEX);
    assert_eq!(count(&r, "decide"), 3);
    for (k, c) in cases(&r).iter().enumerate() {
        let t0 = k as f64 * 24.0 * H;
        let d = row(&r, c.id, "decide");
        assert_eq!(d.enqueue, t0 + 2.0 * H);
        // the third token is absorbed; the case ends when it has arrived
        assert_eq!(c.end, Some(t0 + 3.0 * H));
        assert_eq!(c.end_event.as_deref(), Some("e"));
    }
    assert!(r.outcome.warnings.is_empty(), "{:?}", r.outcome.warnings);
}

#[test]
fn complex_join_with_activation_on_the_last_token() {
    let r = run(&COMPLEX.replace(">= 2", ">= 3"));
    assert_eq!(count(&r, "decide"), 3);
    assert_eq!(row(&r, 1, "decide").enqueue, 3.0 * H);
    assert_eq!(cases(&r)[0].end, Some(3.5 * H));
}

#[test]
fn complex_split_and_join_without_activation_behave_like_or() {
    let text = r#"tok 0.5 "Complex split"

start s
complex which "Which work?"
task fix "Fix" duration="1h"
task typeset "Typeset" duration="2h"
complex merge
task send "Send"
end e

flow s -> which
flow which -> fix "fix" when="fixes"
flow which -> typeset "typeset" default
flow fix -> merge
flow typeset -> merge
flow merge -> send -> e

simulate cases=2 seed=1 arrival="fixed(1d)"
variable fixes "fixed(1)"
"#;
    let r = run(text);
    // only `fix` is chosen; the join waits for that one token
    assert_eq!(trace(&r, 1), ["fix", "send"]);
    assert_eq!(row(&r, 1, "send").enqueue, H);
    let r = run(&text.replace("\"fixed(1)\"", "\"fixed(0)\""));
    assert_eq!(trace(&r, 1), ["typeset", "send"]);
    assert_eq!(row(&r, 1, "send").enqueue, 2.0 * H);
}

// -------------------------------------------------------------- standard loop

fn loop_doc(task: &str) -> String {
    format!(
        "tok 0.5 \"Loop\"\n\nstart s\n{task}\nend e\n\nflow s -> t -> e\n\n\
         simulate cases=1 seed=1\nvariable n \"fixed(0)\"\n"
    )
}

fn iterations(task: &str) -> usize {
    let r = run(&loop_doc(task));
    assert_eq!(cases(&r)[0].end_event.as_deref(), Some("e"));
    count(&r, "t")
}

#[test]
fn standard_loop_counts_iterations() {
    // 0.3 behaviour: a loop marker alone runs once
    assert_eq!(iterations("task t \"T\" loop duration=\"1h\""), 1);
    assert_eq!(iterations("task t \"T\" loop loop-max=3"), 3);
    assert_eq!(iterations("task t \"T\" loop while=\"loopCounter < 4\""), 4);
    assert_eq!(
        iterations("task t \"T\" loop while=\"loopCounter < 4\" loop-max=2"),
        2
    );
    // test after: at least once; test before: possibly never
    assert_eq!(iterations("task t \"T\" loop while=\"false\""), 1);
    assert_eq!(
        iterations("task t \"T\" loop while=\"false\" test-before"),
        0
    );
    assert_eq!(iterations("task t \"T\" loop loop-max=0 test-before"), 0);
    assert_eq!(
        iterations("task t \"T\" loop while=\"loopCounter < 2\" test-before"),
        2
    );
}

#[test]
fn standard_loop_condition_sees_the_variables_set_by_the_activity() {
    let r = run(&loop_doc(
        "task t \"T\" loop while=\"n < 1\" duration=\"1h\" assign=\"n=fixed(1)\"",
    ));
    assert_eq!(count(&r, "t"), 1);
    let r = run(&loop_doc(
        "task t \"T\" loop while=\"n = 0\" test-before duration=\"1h\"",
    ));
    // the condition never changes: the case is stopped by the step limit
    assert_eq!(cases(&r)[0].end_event.as_deref(), Some("loop-limit"));
    let r = run(&loop_doc(
        "task t \"T\" loop while=\"n = 0\" test-before loop-max=5 duration=\"1h\"",
    ));
    assert_eq!(count(&r, "t"), 5);
    let ends: Vec<Option<f64>> = rows(&r).iter().map(|x| x.end).collect();
    assert_eq!(
        ends,
        [
            Some(H),
            Some(2.0 * H),
            Some(3.0 * H),
            Some(4.0 * H),
            Some(5.0 * H)
        ]
    );
}

#[test]
fn standard_loop_of_a_sub_process() {
    let text = r#"tok 0.5 "Loop sub"

start s
sub check "Check" loop loop-max=3
  start ks
  task inspect "Inspect" duration="1h"
  end ke
  flow ks -> inspect -> ke
end e

flow s -> check -> e

simulate cases=1 seed=1
"#;
    let r = run(text);
    assert_eq!(count(&r, "inspect"), 3);
    assert_eq!(count(&r, "check"), 3);
    assert_eq!(cases(&r)[0].end, Some(3.0 * H));
}

// ----------------------------------------------------------- multiple events

fn wait_doc(event: &str) -> String {
    format!(
        "tok 0.5 \"Multiple\"\n\nstart s\n{event}\ntask t \"T\"\nend e\n\n\
         flow s -> w -> t -> e\n\nsimulate cases=1 seed=1\n"
    )
}

fn enqueue_of_t(event: &str) -> f64 {
    let r = run(&wait_doc(event));
    row(&r, 1, "t").enqueue
}

#[test]
fn multiple_catch_fires_on_the_first_trigger() {
    assert_eq!(enqueue_of_t("catch w \"W\" timer=PT2H timer=PT1H"), H);
    assert_eq!(
        enqueue_of_t("catch w \"W\" message=\"M\" timer=PT3H delay=\"fixed(1h)\""),
        H
    );
    assert_eq!(
        enqueue_of_t("catch w \"W\" message=\"M\" timer=PT3H delay=\"fixed(5h)\""),
        3.0 * H
    );
}

#[test]
fn parallel_multiple_catch_waits_for_every_trigger() {
    assert_eq!(
        enqueue_of_t("catch w \"W\" timer=PT2H timer=PT1H parallel"),
        2.0 * H
    );
    assert_eq!(
        enqueue_of_t("catch w \"W\" message=\"M\" timer=PT3H delay=\"fixed(1h)\" parallel"),
        3.0 * H
    );
    assert_eq!(
        enqueue_of_t("catch w \"W\" message=\"M\" timer=PT3H delay=\"fixed(5h)\" parallel"),
        5.0 * H
    );
}

#[test]
fn multiple_boundary_event_fires_at_the_earliest_timer() {
    let text = r#"tok 0.5 "Multiple boundary"

start s
task work "Work" duration="3h"
boundary late "Late" on=work timer=PT5H timer=PT1H signal="Stop"
task escalate "Escalate"
end done
end escalated

flow s -> work -> done
flow late -> escalate -> escalated

simulate cases=1 seed=1
"#;
    let r = run(text);
    let w = row(&r, 1, "work");
    assert_eq!(w.status, "interrupted");
    assert_eq!(w.end, Some(H));
    assert_eq!(cases(&r)[0].end_event.as_deref(), Some("escalated"));
}

#[test]
fn multiple_start_event_starts_cases() {
    let text = r#"tok 0.5 "Multiple start"

start s "Mail or form" message="Mail" message="Form"
task t "T"
end e

flow s -> t -> e

simulate cases=4 seed=1
"#;
    let r = run(text);
    assert_eq!(cases(&r).len(), 4);
    assert_eq!(count(&r, "t"), 4);
}

// -------------------------------------------------------------- instantiation

#[test]
fn instantiating_receive_task_starts_cases() {
    let text = r#"tok 0.5 "Instantiate"

receive order "Order" message="Order" instantiate duration="10m"
task ship "Ship" duration="1h"
end e "Shipped"

flow order -> ship -> e

simulate cases=3 seed=1 arrival="fixed(1d)"
"#;
    let r = run(text);
    assert_eq!(cases(&r).len(), 3);
    assert_eq!(trace(&r, 2), ["order", "ship"]);
    let c = &cases(&r)[1];
    assert_eq!(c.start, 24.0 * H);
    assert_eq!(c.end, Some(24.0 * H + 4200.0));
    assert_eq!(c.end_event.as_deref(), Some("e"));
}

#[test]
fn instantiating_event_gateway_starts_cases() {
    let text = r#"tok 0.5 "Instantiate gateway"

eventgw g "Which came first?" instantiate
catch a "A" message="A" delay="fixed(2h)"
catch b "B" message="B" delay="fixed(1h)"
task ta "Handle A"
task tb "Handle B"
end ea
end eb

flow g -> a -> ta -> ea
flow g -> b -> tb -> eb

simulate cases=2 seed=1
"#;
    let r = run(text);
    assert_eq!(cases(&r).len(), 2);
    assert_eq!(count(&r, "tb"), 2);
    assert_eq!(count(&r, "ta"), 0);
    // without delays the event of the first flow starts the case at once
    let r = run(&text
        .replace(" delay=\"fixed(2h)\"", "")
        .replace(" delay=\"fixed(1h)\"", ""));
    assert_eq!(count(&r, "ta"), 2);
    assert_eq!(row(&r, 1, "ta").enqueue, 0.0);
}

// ---------------------------------------------------------------- conformance

fn conformance(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("conformance")
        .join("cases")
        .join(name)
}

#[test]
fn conformance_cases_of_language_05_simulate() {
    for name in [
        "33-transaction.tok",
        "34-complex.tok",
        "35-loop.tok",
        "36-multiple.tok",
        "37-instantiate.tok",
        "39-resources.tok",
    ] {
        let p = conformance(name);
        let text = std::fs::read_to_string(&p)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))
            .replace("\r\n", "\n");
        let (doc, _) = tok_core::load(&text, "full", None);
        let r = simulate(&doc, None, None).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(!cases(&r).is_empty(), "{name}: no cases");
        assert!(
            cases(&r).iter().all(|c| c.end.is_some()),
            "{name}: unfinished cases"
        );
    }
}

#[test]
fn conformance_transaction_is_cancelled_and_compensated() {
    let text = std::fs::read_to_string(conformance("33-transaction.tok"))
        .unwrap_or_default()
        .replace("\r\n", "\n");
    let (doc, _) = tok_core::load(&text, "full", None);
    let r = simulate(&doc, None, None).unwrap_or_else(|e| panic!("{e}"));
    // `ok` is not a variable: the default flow leads to the cancel end event
    assert_eq!(
        trace(&r, 1),
        [
            "pokoj",
            "bilet",
            "zwrot-biletu",
            "zwolnij-pokoj",
            "rezerwacja",
            "informacja"
        ]
    );
    assert!(
        cases(&r)
            .iter()
            .all(|c| c.end_event.as_deref() == Some("e2"))
    );
}

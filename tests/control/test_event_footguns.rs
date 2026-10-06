// Footguns of events and signals in other systems and what warp does (wiki/Footguns.md "Events, signals and channels")
use crate::is;

#[test]
fn an_event_raised_before_its_handler_reaches_it() {
	is!("n=0; emit ping; on ping {n+=1}; n", 1);
}

#[test]
fn a_handler_made_in_a_loop_keeps_its_iteration() {
	is!("fs = 0; for i in 1 to 3 { on click { fs += i } }; emit click; fs", 6);
}

#[test]
fn a_handler_reads_the_current_state() {
	is!("x = 1; seen = 0; on ping { seen = x }; x = 2; emit ping; seen", 2);
}

#[test]
fn a_listener_writing_what_it_watches_does_not_loop() {
	is!("x = 0; on change x { x = x + 1 }; x = 1; x", 2);
}

#[test]
fn removing_a_handler_while_dispatching_skips_no_other() {
	is!("n = 0; h = on tick { n += 1; remove h from listeners of tick }; on tick { n += 10 }; emit tick; emit tick; n", 21);
}

// the DOM hands one mutable event object down its listeners
#[test]
fn each_handler_gets_the_event_as_raised() {
	is!("seen = 0; on e { event.x = 2 }; on e { seen = event.x }; emit e{x:1}; seen", 1);
	is!("seen = 0; on e { event.x += 5; seen = event.x }; emit e{x:1}; seen", 6);
}

// Vue 2 missed some in-place changes, React misses all of them
#[test]
fn a_change_through_a_list_method_runs_the_listeners() {
	is!("n = 0; x = [1]; on change x { n += 1 }; x.add(2); n", 1);
	is!("n = 0; xs = [1]; f() := { global xs; xs.add(3) }; on set xs { n += 1 }; f(); n", 1);
}

// a getter (`b := a * 2`) read in a method's argument is its value, not a function
#[test]
fn a_derived_value_in_a_method_argument_is_its_value() {
	is!("a = 1; b := a * 2; log = []; on change a { log.add([b, a]) }; a = 2; log#1#1", 4);
}

// a listener made in a function changes the program's counter
#[test]
fn a_subscribed_listener_changes_a_main_level_variable() {
	is!("n = 0; watch(s) := on change s { n += 1 }; x = 1; watch(x); x = 2; x = 3; n", 2);
	is!("n = 0; watch(s) := { global n; on change s { n += 1 } }; x = 1; watch(x); x = 2; n", 1);
	is!("n = 0; watch(s) := on change s { n += 1 }; x = 1; for i in 1 to 3 { watch(x) }; x = 2; n", 3);
}

// Node's emitter.on("conect") never fires, silently: a warning naming the near event, and the body does not run
#[test]
fn a_handler_of_an_event_nothing_raises_never_runs() {
	is!("hits = 0; on conect { hits += 1 }; hits", 0);
	is!("ready = false; n = 0; once ready { n += 1 }; ready = true; n", 1);
}

// P156: whenever runs each time its condition becomes true, also as a subscription and as a named listener
#[test]
fn whenever_runs_when_the_condition_becomes_true() {
	is!("x = 0; n = 0; whenever x > 5 { n += 1 }; x = 6; x = 7; x = 3; x = 8; n", 2);
	is!("n = 0; watch(s) := { whenever s > 5 { n += 1 } }; x = 0; watch(x); x = 6; x = 7; x = 3; x = 8; n", 2);
	is!("t = 20; n = 0; alarm = whenever t > 30 { n += 1 }; t = 35; t = 36; t = 1; t = 40; n", 2);
}

// P163: emit (and send without `to`) sends an event, raise and throw are errors only
#[test]
fn emit_sends_events_and_raise_stays_an_error() {
	is!("n = 0; on alarm { n += event.level }; emit alarm{level: 3}; send alarm{level: 4}; n", 7);
	is!("emit nobody listens; 5", 5);
	is!("n = 0; on alarm { n += 1 }; fire alarm; trigger alarm; n", 2);
	is!("fire(x) := x * 2; fire(3)", 6);
	is!("try { raise alarm } else { 7 }", 7);
	crate::common::fails_with("n = 0; on alarm { n += 1 }; raise alarm; n", "alarm");
}

// Handlers that emit each other without a condition never end (a stack overflow at run time): a compile error naming
// the cycle; a condition or a once handler breaks it
#[test]
fn handlers_emitting_each_other_unconditionally_are_an_error() {
	crate::common::fails_with("on a { emit b }; on b { emit a }; emit a", "on a emits b, on b emits a");
	crate::common::fails_with("on ping { emit ping }; emit ping", "on ping emits ping");
	is!("n = 0; on a { n += 1; if n < 3 { emit b } }; on b { emit a }; emit a; n", 3);
	is!("n = 0; once a { n += 1; emit b }; on b { emit a }; emit a; n", 1);
}

// A handler made in a loop subscribes at each pass (a listener leak, unless meant): it works, with a got-it note
#[test]
fn a_handler_made_in_a_loop_gets_a_note() {
	let code = "n = 0; for i in 1 to 3 { on tick { n += 1 } }; emit tick; n";
	let (result, hints) = warp::normalize::capture_hints(|| warp::wasm_emitter::eval(code));
	assert_eq!(result.serialize(), "3");
	assert!(hints.iter().any(|hint| hint.reason.contains("subscribes once per pass")), "{hints:?}");
	let (_, hints) = warp::normalize::capture_hints(|| warp::wasm_emitter::eval("n = 0; on tick { n += 1 }; emit tick; n"));
	assert!(hints.iter().all(|hint| !hint.reason.contains("once per pass")), "{hints:?}");
}

// Vue's watch(() => b + c): `on change b + c {…}` watches the expression, as `total := b + c; on change total` does
#[test]
fn a_listener_on_an_expression_watches_its_value() {
	is!("b = 1; c = 2; n = 0; on change b + c { n += 1 }; b = 5; c = 2; b = 7; n", 2);
	is!("b = 1; c = 2; seen = 0; on change b * c { seen = b * c }; c = 10; seen", 10);
	is!("b = 1; c = 2; n = 0; on set b + c { n += 1 }; b = 5; c = 3; n", 2);
}

// Unix coalesces two SIGCHLD into one handler call: `on set` of a shared value runs once per write, also for writes
// a task makes between two polls
#[test]
fn on_set_of_a_shared_value_sees_every_write() {
	is!("shared n = 0; count = 0; on set n { count += 1 }; job = go { for i in 1 to 5 { n += 1 } }; await job; sleep(50 ms); count", 5);
	is!("shared n = 0; last = 0; on set n { last = n }; job = go { for i in 1 to 3 { n = i * 10 } }; await job; sleep(50 ms); last", 30);
}

// card whenever-without: a one-line function's whenever (or once) without braces around it subscribes like the braced one
#[test]
fn a_one_line_function_listener_subscribes() {
	is!("n = 0; watch(s) := whenever s > 5 { n += 1 }; x = 0; watch(x); x = 6; x = 3; x = 8; n", 2);
	is!("n = 0; watch(s) := once s > 5 { n += 1 }; x = 0; watch(x); x = 6; x = 3; x = 8; n", 1);
	is!("n = 0; watch(s) := whenever s > 5 { n += 1 }; x = 0; n", 0);
}

// found by the kitchen sink: a call with a lambda argument nested the whole program in a block, so the handler's
// write to a main-level variable was refused ("declare it global")
#[test]
fn a_handler_next_to_a_function_taking_a_lambda() {
	is!("twice = f => f(1); y = twice(x => x + 3); alarms = 0; on alarm { alarms += 1 }; emit alarm; alarms", 1);
}

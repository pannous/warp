// Reactive and event patterns of other systems in warp's signal syntax (notes/signals.md, probes/reactive_ports.md):
// Svelte, Vue, SolidJS, RxJS, C# events, Node EventEmitter, Qt signals/slots, DOM addEventListener
use crate::is;

#[test]
fn svelte_vue_solid_derivations() {
	is!("count = 0; doubled := count * 2; count = 3; doubled", 6);
	is!("count = 0; alerts = 0; whenever count >= 10 {alerts += 1}; count = 5; count = 10; count = 11; alerts", 1); // P156: when it becomes true
	is!("first = \"Ada\"; last = \"L\"; full := first + \" \" + last; first = \"Grace\"; full", "Grace L");
	is!("a = 2; square := a*a; sum = 0; on change square {sum += value}; a = 3; a = 3; a = 4; sum", 25);
}

// Vue watch of a field (`watch(() => p.age, …)`): a write of the field or of the whole object, not of another field
#[test]
fn a_listener_watches_one_field() {
	is!("p = {age: 1}; seen = 0; on set p.age {seen = value}; p.age = 7; seen", 7);
	is!("p = {age: 1, name: \"a\"}; n = 0; on set p.age {n += 1}; p.name = \"b\"; p.age = 3; p = {age: 4, name: \"c\"}; n", 2);
	is!("p = {age: 1}; n = 0; on set p {n += 1}; p.age = 3; n", 1);
}

#[test]
fn rxjs_node_qt_events() {
	is!("total = 0; on price {total += event.value}; emit price{value: 5}; emit price{value: 7}; total", 12);
	is!("evens = 0; on number {if event.n % 2 == 0 {evens += 1}}; for i in 1 to 6 {emit number{n: i}}; evens", 3);
	is!("sum = 0; on add {sum += event.a + event.b}; emit add{a: 1, b: 2}; sum", 3);
	is!("n = 0; on pressed {emit clicked}; on clicked {n += 1}; emit pressed; n", 1);
	is!("seen = 0; def show(v){ global seen; seen = v }; x = 0; on set x : show(value); x = 9; seen", 9);
}

// RxJS unsubscribe, C# `-=`: a named listener changes the program's variables as an unnamed one does (P124)
#[test]
fn a_named_listener_is_removed_and_shares_variables() {
	is!("x = 0; n = 0; s = on set x {n += 1}; x = 1; remove s from listeners of x; x = 2; n", 1);
	is!("x = 0; n = 0; h = on set x {n += 1}; x = 1; listeners of x -= h; x = 2; n", 1);
	is!("x = 0; n = 0; h = on change x {n += value}; x = 4; x = 4; n", 4);
}

// Node's emitter.once, DOM addEventListener(…, {once: true}): `once alarm {…}` runs at the first raise only
#[test]
fn a_once_handler_runs_at_the_first_raise() {
	is!("n = 0; once alarm {n += 1}; emit alarm; emit alarm; n", 1);
	is!("n = 0; once alarm {n += 1}; on alarm {n += 10}; emit alarm; emit alarm; n", 21);
	is!("n = 0; def f() { emit ping }; once ping {n += 1}; f(); f(); n", 1);
	is!("level = 0; once alarm {level = event.level}; emit alarm{level: 3}; emit alarm{level: 5}; level", 3);
}

// Vue's watch(x, (value, old) => …), Qt's valueChanged with the previous value: `old` in `on change x` is x before
#[test]
fn a_change_listener_reads_the_old_value() {
	is!("x = 1; diff = 0; on change x {diff = value - old}; x = 5; diff", 4);
	is!("x = 1; log = 0; on change x {log = log*10 + previous}; x = 2; x = 3; log", 12);
	is!("watch(s) := on change s {print old}\nx = 1\nwatch(x)\nx = 7\nx", 7);
}


// the example of notes/signals.md: a derived total, a whenever printing with juxtaposed words
#[test]
fn the_signals_note_example_runs() {
	is!("price = 3; count = 2; total := price * count; whenever total > 10 { print \"big order: \" total }; count = 5; total", 15);
	is!("price = 3\ncount = 2\ntotal := price * count\nwhenever total > 10 { print \"big order: \" total }\non change total { print \"total is now \" value }\ncount = 5\ntotal", 15);
}

// P148: `old` in `on set x` too (the value before this write), `previous`, `was`, `before` its aliases with a note;
// a program variable of that name keeps its meaning
#[test]
fn old_in_set_listeners_and_its_aliases() {
	is!("x = 1; log = 0; on set x {log = log*10 + old}; x = 2; x = 3; log", 12);
	is!("p = {age: 1}; d = 0; on set p.age {d = value - old}; p.age = 7; d", 6);
	is!("x = 1; d = 0; on change x {d = value - was}; x = 5; d", 4);
	is!("x = 1; d = 0; on change x {d = value - before}; x = 5; d", 4);
	is!("old = 9; x = 1; on change x {print old}; x = 2; old", 9);
	is!("watch(s) := on set s {print old}\nx = 1\nwatch(x)\nx = 7\nx", 7);
}

// Node's emitter.off, DOM removeEventListener, C# -=, listenerCount: a named event handler can be removed (P128)
#[test]
fn a_named_event_handler_is_removed_and_counted() {
	is!("n = 0; h = on alarm {n += 1}; emit alarm; remove h from listeners of alarm; emit alarm; n", 1);
	is!("n = 0; h = on alarm {n += 1}; emit alarm; listeners of alarm -= h; emit alarm; n", 1);
	is!("n = 0; h = on stop the machine {n += 1}; emit stop the machine; remove h from listeners of stop the machine; emit stop the machine; n", 1);
	is!("on tick {1}; on tick {2}; count listeners of tick", 2);
	is!("on tick {1}; h = on tick {2}; remove h from listeners of tick; count listeners of tick", 1);
	is!("n = 0; h = on ready {n += 1}; remove h from listeners of ready; emit ready; n", 0);
	// EventEmitter removeAllListeners, Qt disconnect(): every handler stops, the raise reaches none
	is!("n = 0; on alarm {n += 1}; on alarm {n += 10}; emit alarm; clear listeners of alarm; emit alarm; n", 11);
	is!("on alarm {1}; on alarm {2}; clear listeners of alarm; count listeners of alarm", 0);
	// RxJS take(2): the handler removes itself
	is!("taken = 0; h = on tick {taken += 1; if taken == 2 {remove h from listeners of tick}}; for i in 1 to 5 {emit tick}; taken", 2);
}

// EventEmitter's emit("item", 1), RxJS subject.next(v): a value after the event's words is its data
#[test]
fn a_raise_carries_a_plain_value() {
	is!("xs = []; on item {xs = xs + [event]}; emit item 1; emit item 2; count xs", 2);
	is!("n = 0; on item {n += event}; emit item 5; emit item 6; n", 11);
	is!("msg = \"\"; on said {msg = event}; emit said \"hi\"; msg", "hi");
}

// EventEmitter.on in a loop: a handler inside a block subscribes each time the block runs, with that pass's values
#[test]
fn a_handler_in_a_block_subscribes_each_pass() {
	is!("hits = 0; for i in 1 to 3 { on tick2 {hits += 1} }; emit tick2; hits", 3);
	is!("log = 0; for i in 1 to 3 { on tick {log = log*10 + i} }; emit tick; log", 123);
	is!("n = 0; on alarm {n += 100}; if 1 { on alarm {n += event} }; emit alarm 5; n", 105);
}

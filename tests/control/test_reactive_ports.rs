// Reactive and event patterns of other systems in wasp's signal syntax (notes/signals.md, probes/reactive_ports.md):
// Svelte, Vue, SolidJS, RxJS, C# events, Node EventEmitter, Qt signals/slots, DOM addEventListener
use crate::is;

#[test]
fn svelte_vue_solid_derivations() {
	is!("count = 0; doubled := count * 2; count = 3; doubled", 6);
	is!("count = 0; alerts = 0; whenever count >= 10 {alerts += 1}; count = 5; count = 10; count = 11; alerts", 2);
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
	is!("total = 0; on price {total += event.value}; raise price{value: 5}; raise price{value: 7}; total", 12);
	is!("evens = 0; on number {if event.n % 2 == 0 {evens += 1}}; for i in 1 to 6 {raise number{n: i}}; evens", 3);
	is!("sum = 0; on add {sum += event.a + event.b}; raise add{a: 1, b: 2}; sum", 3);
	is!("n = 0; on pressed {raise clicked}; on clicked {n += 1}; raise pressed; n", 1);
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
	is!("n = 0; once alarm {n += 1}; raise alarm; raise alarm; n", 1);
	is!("n = 0; once alarm {n += 1}; on alarm {n += 10}; raise alarm; raise alarm; n", 21);
	is!("n = 0; def f() { raise ping }; once ping {n += 1}; f(); f(); n", 1);
	is!("level = 0; once alarm {level = event.level}; raise alarm{level: 3}; raise alarm{level: 5}; level", 3);
}

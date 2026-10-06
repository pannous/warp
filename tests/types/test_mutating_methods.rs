// A method changing fields of its object (user decision P116, notes/classes.md step 2): `c.inc()` updates the variable c,
// as `xs.add(v)` updates xs; the call gives the changed object
use crate::is;

#[test]
fn a_method_changing_a_field_updates_the_variable() {
	is!("class counter{n:int; inc() := n += 1}; c = counter(0); c.inc(); c.inc(); c.n", 2);
	is!("class counter{n:int; add(k) := self.n = n + k}; c = counter(1); c.add(5); c.n", 6);
	is!("class point{x:int y:int; move(dx, dy) := { x += dx; y += dy }}; p = point(1, 2); p.move(10, 20); p.x + p.y", 33);
}

#[test]
fn the_call_gives_the_changed_object() {
	is!("class counter{n:int; inc() := n += 1}; counter(4).inc().n", 5);
	is!("class counter{n:int; inc() := n += 1}; c = counter(0); d = c.inc(); d.n + c.n", 2);
}

#[test]
fn pop_on_a_list_field() {
	// card obj-list: a list in a field changes like a list in a variable
	is!("s = {xs: [1 2 3]}; s.xs.pop()", 3);
	is!("s = {xs: [1 2 3]}; s.xs.pop(); string(s.xs)", "[1 2]");
	is!("s = {listeners: []}; s.listeners.push(3); string(s.listeners)", "[3]");
}

#[test]
fn a_method_changes_its_object_and_gives_a_value() {
	let stack = "class Stack{items:list; push(x) := items.add(x); pop() := items.pop()}; ";
	is!(&format!("{stack}s = Stack([1 2]); s.pop()"), 2);
	is!(&format!("{stack}s = Stack([1 2]); s.pop(); string(s.items)"), "[1]");
	is!(&format!("{stack}s = Stack([]); s.push(5); s.push(6); s.pop() * 10 + s.pop()"), 65);
}

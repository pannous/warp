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

#[test]
fn a_block_method_changes_its_object_and_gives_a_value() {
	// a Queue's dequeue: statements before the value it gives
	let queue = "class Queue{items:list; dequeue() := { x = items#1; items = items[1:]; x }}; ";
	is!(&format!("{queue}q = Queue([4 5 6]); q.dequeue() * 10 + q.dequeue()"), 45);
	is!(&format!("{queue}q = Queue([4 5 6]); q.dequeue(); count(q.items)"), 2);
}

#[test]
fn a_method_changing_its_object_inside_if_gives_the_object() {
	// a Set's add: the change happens only on one branch, the method still gives its object
	let set = "class Set{items:list; put(x) := { if not (x in items) { items.add(x) } }}; ";
	is!(&format!("{set}s = Set([]); s.put(1); s.put(1); s.put(2); count(s.items)"), 2);
	let either = "class Set{items:list; put(x) := { if x in items { items } else { items = items + [x] } }}; ";
	is!(&format!("{either}s = Set([]); s.put(1); s.put(1); count(s.items)"), 1);
}

#[test]
fn a_method_setting_an_element_of_a_list_field_changes_its_object() {
	is!("class C{counts:list; bump(i) := { counts#i = counts#i + 1 }}; c = C([5 5]); c.bump(2); c.counts#2", 6);
	is!("class C{counts:list; bump(i) := { counts#i += 1 }}; c = C([5 5]); c.bump(2); c.counts#2", 6);
}

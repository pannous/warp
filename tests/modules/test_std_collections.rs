//! The standard library module collections (notes/stdlib.md, notes/classes.md): classes written in wasp
use crate::is;

#[test]
fn a_stack_pushes_and_pops_the_last() {
	is!("use collections; s = Stack(); s.push(1); s.push(2); s.pop() * 10 + s.size()", 21);
	is!("use collections; s = Stack(); s.push(5); s.peek()", 5);
}

#[test]
fn a_queue_gives_the_first() {
	is!("use collections; q = Queue(); q.enqueue(4); q.enqueue(5); q.dequeue() * 10 + q.size()", 41);
}

#[test]
fn a_deque_works_at_both_ends() {
	is!("use collections; d = Deque(); d.push_back(2); d.push_front(1); d.push_back(3); d.pop_front() * 10 + d.pop_back()", 13);
}

#[test]
fn a_set_keeps_each_value_once() {
	is!("use collections; s = Set([1 2 2 3]); s.size()", 3);
	is!("use collections; s = Set(); s.add(1); s.add(1); s.add(2); s.size()", 2);
	is!("use collections; s = Set([1 2]); s.remove(1); s.has(1) or s.size() != 1", false);
	is!("use collections; s = Set([\"ab\" \"cd\"]); s.has(\"cd\")", true);
}

#[test]
fn a_counter_counts() {
	is!("use collections; c = Counter([7 8 7]); c.get(7) * 10 + c.get(9)", 20);
	is!("use collections; c = Counter(); c.add(\"ab\"); c.add(\"ab\"); c.get(\"ab\")", 2);
	is!("use collections; c = Counter([7 8 8]); c.most_common()", 8);
}

#[test]
fn a_collection_without_its_use_names_the_module() {
	crate::common::fails_with("s = Stack(); s.push(1)", "Stack is in the standard module collections: write `use collections`");
}

#[test]
fn foreign_spellings_of_a_collection() {
	is!("use collections; s = new Set([1 2 2]); s.size()", 2);
	is!("use collections; c = collections.Counter([1 1]); c.get(1)", 2);
}

#[test]
fn other_languages_class_names_are_the_collections() {
	is!("use collections; s = HashSet([1 2 2]); s.size()", 2);
	is!("use collections; d = ArrayDeque(); d.push_back(4); d.pop_front()", 4);
	is!("use collections; d = deque(); d.push_front(5); d.size()", 1);
	// a program's own class of that name wins
	is!("use collections; class HashSet{n=7}; HashSet().n", 7);
}

#[test]
fn other_languages_method_names_are_the_collections_methods() {
	// Python's deque
	is!("use collections; d = deque(); d.append(1); d.appendleft(0); d.popleft() * 10 + d.pop()", 1);
	// Java's ArrayDeque and Queue
	is!("use collections; d = ArrayDeque(); d.addLast(2); d.addFirst(1); d.pollFirst() * 10 + d.pollLast()", 12);
	is!("use collections; q = Queue(); q.offer(4); q.offer(5); q.poll()", 4);
	// JS's Set and Java's contains
	is!("use collections; s = Set([1 2]); s.delete(1); s.contains(2) and not s.has(1)", true);
	// a stack's append and JS's shift on a queue
	is!("use collections; s = Stack(); s.append(3); s.pop()", 3);
	is!("use collections; q = Queue([7 8]); q.shift()", 7);
}

#[test]
fn the_size_of_an_instance_through_len_and_count() {
	is!("use collections; s = Set([1 2 2]); len(s)", 2);
	is!("use collections; s = Stack(); s.push(1); count(s) + s.len() + s.count()", 3);
}

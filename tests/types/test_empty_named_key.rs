//! card object-empty-field: a key named like the empty value (`empty`, `none`, `nothing`) is a name, `{empty: yes}`
//! keeps the key empty; a spaced ternary `c ? empty : 0` still reads the value
use crate::is;

#[test]
fn a_key_named_empty_is_a_field() {
	is!("x = {empty: yes}; x.empty", true);
	is!("x = {empty: no, next: {empty: yes}}; x.next.empty", true);
	is!("x = {none: 1, nothing: 2}; x.none + x.nothing", 3);
	is!("class Cell{empty: bool}\nc = Cell(yes)\nc.empty", true);
}

#[test]
fn a_linked_list_of_empty_marked_cells() {
	let program = "def append(l, v) = if l.empty : {empty: no value: v next: {empty: yes}} else {empty: no value: l.value next: append(l.next, v)}
lst = {empty: yes}
for i in 1 to 4 { lst = append(lst, i) }
s = \"\"
cur = lst
while not cur.empty {
	if s.length > 0 : s += \" \"
	s += \"$(cur.value)\"
	cur = cur.next
}
s";
	is!(program, "1 2 3 4");
}

#[test]
fn a_spaced_ternary_still_reads_the_empty_value() {
	is!("c = no; x = c ? 1 : empty; x", warp::Node::Empty);
}

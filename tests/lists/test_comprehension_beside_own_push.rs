//! card prefix-count-field: a comprehension appends with `+=`, so a program's own `push(st, x)` (a stack's) does not
//! answer the comprehension's appends; `#st.items` after it stays the length
use crate::is;

const STACK: &str = "class Stack { items: [int] }
s = Stack{items: []}
def push(st, x) {
	st.items += [x]
}
def pop(st) {
	n = #st.items
	top = st.items#n
	st.items = [st.items#i for i in 1 to n - 1]
	top
}
";

#[test]
fn a_comprehension_beside_an_own_push_function() {
	is!(&format!("{STACK}push(s, 1)\npush(s, 2)\npush(s, 3)\npop(s)"), 3);
	is!(&format!("{STACK}push(s, 1)\npush(s, 2)\npush(s, 3)\npop(s)\nn = #s.items\nn"), 2);
}

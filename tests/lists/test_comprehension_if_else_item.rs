// card comprehension-whose: a comprehension whose item is an if-then-else ending in a name, `else c for c in t`; the
// else branch took `for c in t` as an argument of `c` and the list printed [ø]
use crate::is;
use warp::warp_parser::parse;

#[test]
fn an_else_branch_ending_in_a_name_leaves_the_for_clause_to_the_list() {
	is!("t = \"ab c\"; [if c == ' ' then '_' else c for c in t]", parse("['a' 'b' '_' 'c']"));
	is!("[if x > 1 then 0 else x for x in [1 2 3]]", parse("[1 0 0]"));
	is!("[if x > 1 then x else 0 for x in [1 2 3]]", parse("[0 2 3]"));
}

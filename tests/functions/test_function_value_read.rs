use crate::is;

#[test]
fn a_local_read_from_a_nested_function_list_is_callable() {
	is!("inc(x: int): int := x + 1; fs = [inc]; def applied(n: int): int { for functions in [fs] { callback = functions#1; n = callback(n) }; n }; applied(2)", 3);
	is!("inc(x: int): int := x + 1; fs = [inc]; groups = [fs]; def applied(n: int): int { for functions in groups { callback = functions#1; n = callback(n) }; n }; applied(2)", 3);
}

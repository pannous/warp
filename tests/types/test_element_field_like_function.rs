//! card element-field-like-function: `x.kinds` reads the field kinds of an instance when the program also defines a
//! function kinds(x), where the instance's class is known only at run time (a map element) or only in its own function
use crate::is;

const SCOPE: &str = "class S { kinds: map = {} }\nkinds(program: list): int := 7\n";

#[test]
fn a_field_of_a_map_element_named_like_a_function() {
	is!(&format!("{SCOPE}scopes = {{}}\nscopes[\"a\"] = S()\n#scopes[\"a\"].kinds"), 0);
}

// card field-read: the index of a variable key, `scopes#((string k) + 1)`, is read twice by the member test
#[test]
fn a_field_of_a_map_element_by_a_variable_key() {
	is!(&format!("{SCOPE}scopes = {{}}\nk = \"a\"\nscopes[k] = S()\n#scopes[k].kinds"), 0);
	is!(&format!("{SCOPE}xs = {{}}\nk = \"a\"\nxs[k] = [1]\nxs[k].kinds"), 7);
}

// another function's `scope` of unknown value leaves the class of this function's `scope = S()` known: a construction
// reads as its class's fields
#[test]
fn a_field_of_a_constructed_local_named_like_a_function() {
	is!(CONSTRUCTED_LOCAL, 1);
}

const CONSTRUCTED_LOCAL: &str = r#"class S { kinds: map = {}; pinned: map = {} }
kinds(program: list): int := 7
global scopes = {}
size(scope: S): int := #scope.kinds
apply(c: text): int := {
	scope = scopes[c]
	#scope.kinds
}
analyze(names: list): int := {
	scopes[""] = S()
	for name in names {
		scope = S()
		scope.pinned = scopes[""].kinds
		for p in names {
			own = p
			scope.kinds[own] = "int"
		}
	}
	1
}
analyze(["a"])"#;

#[test]
fn the_function_of_a_map_element_holding_no_such_field() {
	is!(&format!("{SCOPE}xs = {{}}\nxs[\"a\"] = [1]\nxs[\"a\"].kinds"), 7);
}

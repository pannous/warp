// card symbol-parameters: a `symbol` parameter holds a symbol and takes a bare word as one, so `form post "/todos"` calls
// form(method: symbol, action: text) (decision markup-plain-library)
use crate::common::fails_with;
use crate::is;

#[test]
fn a_symbol_parameter_takes_a_symbol() {
	is!("f(m: symbol) := m == :post; f(:post)", true);
	is!("f(m: symbol) := m; y = :post; f(y) == :post", true);
	is!("\"post\" as symbol == :post", true);
}

#[test]
fn a_symbol_parameter_takes_a_bare_word() {
	is!("f(m: symbol) := m == :post; f(post)", true);
	is!(r#"form(method: symbol, action: text) := "\(method) \(action)"; form post "/todos""#, "post /todos");
}

#[test]
fn a_symbol_parameter_refuses_a_number() {
	fails_with("f(m: symbol) := m; z = 3; f(z)", "needs a Symbol for parameter m");
}

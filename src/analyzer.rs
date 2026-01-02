use crate::node::Node;
use crate::type_kinds::AstKind;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use wasm_ast::Function;

// use once_cell::unsync::Lazy;

pub static FUNCTIONS: Lazy<HashMap<String, Function>> = Lazy::new(|| {
	let m = HashMap::new();
	// m.insert("foo".into(), Function { ... });
	m
});

pub fn analyze(raw: Node) -> Node {
	//Node::Ast {
	raw
	// todo!()
}

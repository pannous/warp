use warp::wasm_emitter::eval;
use warp::{is, Node};

// every function definition form infers a list parameter from its indexing use
#[test]
fn list_parameters_inferred_in_every_definition_form() {
	is!("get(xs) := xs#2; q=[0 4 0]; get(q)", 4);
	is!("fn get(xs) { xs#2 }; q=[0 4 0]; get(q)", 4);
	is!("def get(xs) { xs#2 }; q=[0 4 0]; get(q)", 4);
	is!("def get(xs): xs#2; q=[0 4 0]; get(q)", 4);
	is!("function get(xs) { xs#2 }; q=[0 4 0]; get(q)", 4);
	is!("fn get(xs) { xs[1] }; q=[0 4 0]; get(q)", 4);
	is!("fn get(xs, i) { xs[i] }; q=[0 4 0]; get(q, 1)", 4);
	is!("def get(xs) { return xs[1] }; q=[0 4 0]; get(q)", 4);
	is!("fn total(xs) { s=0; for x in xs { s+=x }; s }; total([1 3 0])", 4);
	is!("cells=[]; cells.push(0); cells.push(4); fun f(g,i){return g[i]}; f(cells,1)", 4);
	is!("fun mk() { return [0 4 0] }; fun g(xs) { xs[1] }; g(mk())", 4);
	is!("fun f2(grid) { grid[1] }; fun f(grid) { f2(grid) }; f([0 4 0])", 4);
	is!("fun f(items = []) { items.push(4); items[0] }; f()", 4);
}

// counting, walking or forwarding a parameter also makes it a list, even when the argument is a call result
#[test]
fn list_parameters_inferred_from_counting_walking_forwarding() {
	is!("f(xs) := count(xs); nums=[1 2 3 4]; f(nums)", 4);
	is!("fun f(xs) { xs.length }; fun mk() { return [1 2 3 4] }; f(mk())", 4);
	is!("fun f(xs) { n = #xs; n }; fun mk() { return [1 2 3 4] }; f(mk())", 4);
	is!("fun f(xs) { s=0; for x in xs { s+=x }; s }; fun mk() { return [1 3] }; f(mk())", 4);
	is!("fun f(xs = []) { count(xs) }; f([1 2 3 4])", 4);
	is!("fun n(xs) { count(xs) }; fun f(xs) { n(xs) }; fun mk() { return [1 2 3 4] }; f(mk())", 4);
}

// a list annotation on a parameter: `xs:list`, `xs:[int]`, `xs:int[]`, `int[] xs`, `xs:list<int>`, plurals `xs:ints`, `xs:numbers`
#[test]
fn list_annotations_on_parameters() {
	is!("fn get(xs:list) { xs#2 }; get([0 4 0])", 4);
	is!("fn get(xs:[int]) { xs#2 }; get([0 4 0])", 4);
	is!("fn get(xs:int[]) { xs#2 }; get([0 4 0])", 4);
	is!("fn get(int[] xs) { xs#2 }; get([0 4 0])", 4);
	is!("fn get(xs:list<int>) { xs#2 }; get([0 4 0])", 4);
	is!("fn get(xs:ints) { xs#2 }; get([0 4 0])", 4);
	is!("fn get(xs:numbers) { xs#2 }; get([0 4 0])", 4);
	is!("get(xs:list) := xs[1]; get([0 4 0])", 4);
}

// a declared list refuses a number or a text loudly (wiki/Footguns.md "Type annotations not enforced loudly")
#[test]
fn declared_list_parameters_refuse_other_kinds() {
	for (code, refusal) in [
		("fn f(xs:list){count(xs)}; f(\"abc\")", "f needs a List for parameter xs, got \"abc\" (a Text)"),
		("fn f(xs:ints){xs#1}; f(5)", "f needs a List for parameter xs, got 5 (an Int)"),
	] {
		match eval(code) {
			Node::Error(message) => assert!(format!("{message}").contains(refusal), "{code}: {message}"),
			other => panic!("{code} should be refused, got {other:?}"),
		}
	}
}

// a parameter shadows an outer variable of the same name: its kind comes from its own calls
#[test]
fn parameters_shadow_literal_variables() {
	is!("s=\"abc\"; fn h(s){ s#1 }; fn g(s){ h(s) }; g([4 5])", 4);
}

// an indexed parameter is a sequence: the text it is passed (literal or variable) or its annotation wins over the list default
#[test]
fn indexed_text_parameters() {
	is!("fun f(a) { return a[1] }; f(\"abcd\")", 'b');
	is!("fun f(a) { return a#2 }; s=\"abcd\"; f(s)", 'b');
	is!("fun f(a: text) { return a[1] }; f(\"abcd\")", 'b');
	is!("fun f(a: string) { a#2 }; s=\"abcd\"; f(s)", 'b');
}

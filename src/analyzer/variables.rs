//! Variables and scope: collecting a program's variables, main-level bindings, the Scope of locals

use super::*;
use crate::words::{ANY_TYPE, FLOAT_WORD, LET_WORD, MAP_WORD};
use std::collections::HashSet;

/// `var x = 1` announces a reassignable variable, plain `x = 1`
pub(crate) const VAR_KEYWORD: &str = "var";

/// Collect variables defined in node and populate scope
/// Returns count of temp locals needed (e.g., for while loops)
pub fn collect_variables(node: &Node, scope: &mut Scope) -> u32 {
	let before = scope.clone();
	let mut split_globals: Vec<Local> = vec![];
	loop {
		let seeded = scope.widened_bindings.clone();
		let temporaries = collect_variables_inner(node, scope, false, false);
		// `b = 0; global b = 1`: a variable written before its `global` declaration is that global throughout, never a
		// local of the same name besides it; collected again with it global from the start
		let split: Vec<Local> = scope
			.globals
			.values()
			.filter(|global| scope.locals.contains_key(&global.name) && !before.locals.contains_key(&global.name))
			.cloned()
			.collect();
		// `xs = [0.0, 0.0]; … xs#i = √2`: a list's element type joins its literal and every write to it, so a read before
		// the widening write (`m = xs#i`) has the widened type too; collected again with that type from the start
		if split.is_empty() && scope.widened_bindings == seeded {
			return temporaries;
		}
		let widened = std::mem::take(&mut scope.widened_bindings);
		split_globals.extend(split);
		*scope = before.clone();
		scope.globals.extend(split_globals.iter().map(|global| (global.name.clone(), global.clone())));
		scope.widened_bindings = widened;
	}
}

/// The binding a `global` declaration introduces: `global x` (an Int) and `global x=7`; it has no local slot
pub(super) fn global_binding(declaration: &Node, scope: &Scope) -> Option<Local> {
	let (name, value) = match declaration.drop_meta() {
		Node::Symbol(name) => (name, &Node::Number(Number::Int(0))),
		Node::Key(target, Op::Assign | Op::Define, value) => match target.drop_meta() {
			Node::Symbol(name) => (name, value.as_ref()),
			_ => return None,
		},
		_ => return None,
	};
	let (kind, type_node) = value_binding(value, scope);
	Some(Local { type_node, ..Local::new(0, name.clone(), kind) })
}

/// Kind and type of a variable bound to `value` without a declared type; a list keeps its element type
pub(super) fn value_binding(value: &Node, scope: &Scope) -> (Kind, Option<Box<Node>>) {
	let kind = binding_kind(value, scope);
	let type_name = match kind {
		Kind::List => Some(list_type_name(value, scope)),
		Kind::Int if is_boolean(value, scope) => Some(BOOL_TYPE.to_string()),
		// `x = 255 as hex`, `x = 255 as byte`: x shows in hex too (card hex-printing), without the byte's range
		Kind::Int
			if let Node::Key(_, Op::As, target) = value.drop_meta()
				&& let Some(radix) = crate::wasm_emitter::numbers::radix::shown_radix(&target.name()) =>
		{
			Some(radix.word.to_string())
		}
		_ => None,
	};
	(kind, type_name.map(|name| Box::new(Node::Symbol(name))))
}

/// The body of a function definition `f(x) = …`, `f(x) := …`, `def f(x) {…}`: its variables are the function's own
pub(super) fn function_definition_body(node: &Node) -> Option<&Node> {
	let starts_with_symbol = |items: &[Node]| matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_)));
	match node.drop_meta() {
		Node::Key(left, Op::Assign | Op::Define, body) if matches!(left.drop_meta(), Node::List(items, _, _) if starts_with_symbol(items)) => {
			Some(body)
		}
		Node::List(items, _, _)
			if items.len() >= 2
				&& matches!(items[0].drop_meta(), Node::Symbol(keyword) if is_function_keyword(keyword)) =>
		{
			items.last()
		}
		_ => None,
	}
}

pub(super) fn collect_variables_inner(
	node: &Node,
	scope: &mut Scope,
	skip_first_assign: bool,
	in_structure: bool,
) -> u32 {
	let node = node.drop_meta();
	if function_definition_body(node).is_some() {
		return 0;
	}
	let visit = |child: &Node, scope: &mut Scope| collect_variables_inner(child, scope, false, in_structure);
	if let Some((names, values)) = crate::tuples::destructuring(node) {
		let temporaries = values.iter().map(|value| visit(value, scope)).sum();
		bind_destructured(&names, values, scope);
		return temporaries;
	}
	match node {
		// `c ? a : b`: both branches are code, never a tag `a:b` whose body holds attributes
		Node::Key(condition, Op::Question, then_else)
			if let Node::Key(then, Op::Colon, otherwise) = then_else.drop_meta() =>
		{
			visit(condition, scope) + visit(then, scope) + visit(otherwise, scope)
		}
		// `global:xs = …` defines no local, but counts the variables in the value expression first: the kind of a
		// hoisted block `global:xs = (made = ø; …; made)` is the kind of its temporaries
		Node::Key(left, Op::Colon, right) if left.is_symbol("global") => {
			let count = collect_variables_inner(right, scope, true, false);
			if let Some(global) = global_binding(right, scope).filter(|global| !scope.is_global(&global.name)) {
				scope.globals.insert(global.name.clone(), global);
			}
			count
		}
		// a tag `html:body`: the body is structure context, where `=` sets an attribute, not a variable
		Node::Key(left, Op::Colon, right) if left.symbol_name().is_some() => {
			visit(left, scope) + collect_variables_inner(right, scope, false, true)
		}
		// Define (:=) always creates a variable, Assign (=) only outside structure context
		Node::Key(left, Op::Define, right) => {
			if !skip_first_assign {
				define_new(left, right, scope);
			}
			visit(right, scope)
		}
		Node::Key(left, Op::Assign, right) => {
			// the variables the value binds come first, so its kind is known: `a = (t = 1 + 2; t)` makes a an Int
			let inner_temporaries = visit(right, scope);
			if !skip_first_assign && !in_structure {
				bind_assigned(left, right, scope);
			}
			inner_temporaries
		}
		// Compound assignments don't create new variables
		// the value's variables come first, as for `=`: `r += (t = l[b]; t)` (a lowered max) widens r by t's kind
		Node::Key(left, op, right) if op.is_compound_assign() => {
			let inner_temporaries = visit(right, scope);
			widen_by_compound_assign(left, op, right, scope);
			visit(left, scope) + inner_temporaries
		}
		// While loop needs temp locals for the result and for "the body ran"
		Node::Key(left, Op::Do, right) => 2 + visit(left, scope) + visit(right, scope),
		// Integer abs needs a temp local for the if-then-else pattern
		Node::Key(left, Op::Abs, right) if left.is_empty_node() => 1 + visit(right, scope),
		Node::Key(left, _, right) => visit(left, scope) + visit(right, scope),
		// a group of statements in a structure is code: `ul{ (made = ø; for … ; made) }`, a lowered `[li{t} for t in ts]`
		Node::List(items, Bracket::Round, Separator::Semicolon | Separator::Newline) => {
			items.iter().map(|item| collect_variables_inner(item, scope, false, false)).sum()
		}
		Node::List(items, _, _) => items.iter().map(|item| visit(item, scope)).sum(),
		_ => 0,
	}
}

/// `x := v` of a name not yet a local or global defines it
fn define_new(left: &Node, right: &Node, scope: &mut Scope) {
	if let Some(name) = left.symbol_name() {
		if scope.lookup(name).is_none() && !scope.is_global(name) {
			let kind = binding_kind(right, scope);
			scope.define(name.to_string(), None, kind);
		}
	}
}

/// `x += v` widens x by the kind of v: a ratio, a float, a Node of run-time kind, a list by its first append
fn widen_by_compound_assign(left: &Node, op: &Op, right: &Node, scope: &mut Scope) {
	let Some(name) = left.symbol_name() else {
		return;
	};
	mark_ratio(left, right, scope);
	widen_to_float(scope, name, right);
	// `out += x` of a number of run-time kind may add a float, as `out = out + x` does; an evident other kind
	// (`s += "ab"` of an Int) stays the type error of emit_compound_type_error
	if crate::analyzer::inference::is_run_time_kind(&binding_kind(right, scope)) {
		widen_to_node(scope, name, &Node::Key(Box::new(left.clone()), op.base_op(), Box::new(right.clone())));
	}
	// `xs += [v]` (a lowered `xs.push(v)`) types xs as `xs = xs + [v]` does
	if *op == Op::AddAssign {
		type_list_by_first_append(name, &Node::Key(Box::new(left.clone()), Op::Add, Box::new(right.clone())), scope);
	}
}

/// `a, b = values`: each new name a local; with a starred name the values are unpacked from one list, each name
/// holding a Node and the star a list
fn bind_destructured(names: &[String], values: &[Node], scope: &mut Scope) {
	let starred = names.iter().any(|name| crate::tuples::unstarred(name) != name);
	for (index, name) in names.iter().enumerate() {
		let rest = crate::tuples::unstarred(name);
		if scope.lookup(rest).is_none() && !scope.is_global(rest) {
			let kind = match (starred, rest != name) {
				(_, true) => Kind::List,
				(true, false) => Kind::Data,
				(false, false) => destructured_kind(values, index, scope),
			};
			scope.define(rest.to_string(), None, kind);
		}
	}
}

/// `left = right` outside a structure: a new name becomes a local, a known one may widen its type
fn bind_assigned(left: &Node, right: &Node, scope: &mut Scope) {
	bind_assigned_kind(left, right, scope);
	mark_ratio(left, right, scope);
}

/// A variable given a value that may be a fraction keeps that in mind: its power `10^l` may be a float (node_pow)
fn mark_ratio(left: &Node, right: &Node, scope: &mut Scope) {
	let Some(name) = left.symbol_name() else { return };
	if crate::analyzer::inference::may_be_ratio(right, scope) {
		if let Some(local) = scope.own_binding_mut(name) {
			local.may_be_ratio = true;
		}
	}
}

fn bind_assigned_kind(left: &Node, right: &Node, scope: &mut Scope) {
	match left.drop_meta() {
		// `k.x = 3` updates k: it binds no k, so an undefined k stays undefined (as in `k#1 = 3`)
		Node::Symbol(name) if crate::library_words::is_field_update_of(name, right) && scope.lookup(name).is_none() => {
		}
		Node::Symbol(name) if scope.lookup(name).is_none() && !scope.is_global(name) => {
			let declared = declared_type(left);
			let (kind, type_node) = match declared {
				Some(type_name) => (
					declared_kind(type_name).unwrap_or_else(|| binding_kind(right, scope)),
					Some(Box::new(type_name.clone())),
				),
				None => match (value_binding(right, scope), scope.widened_bindings.get(name)) {
					((Kind::List, _), Some(widened @ (Kind::List, _)))
					| ((Kind::Int, None), Some(widened @ (Kind::Float, None))) => widened.clone(),
					((kind, _), Some(widened @ (Kind::Empty, None))) if CONCRETE_KINDS.contains(&kind) => {
						widened.clone()
					}
					(binding, _) => binding,
				},
			};
			scope.define(name.clone(), type_node, kind);
			if let Some(local) = scope.own_binding_mut(name) {
				local.declared = declared.is_some();
			}
		}
		Node::Symbol(name) => {
			type_list_by_first_append(name, right, scope);
			type_map_by_first_entry(name, right, scope);
			widen_list_type(scope, name, right);
			widen_to_float(scope, name, right);
			widen_to_node(scope, name, right);
		}
		Node::Key(list, Op::Hash, _) => widen_element_type(scope, list, right),
		// Typed variable: x:int = 1 parses as Key(Key(x, Colon, int), Assign, 1)
		Node::Key(var_name, Op::Colon, type_node) => {
			if let Some(name) = var_name.symbol_name().filter(|name| scope.lookup(name).is_none()) {
				let kind = declared_kind(type_node).unwrap_or(Kind::Int);
				scope.define(name.to_string(), Some(type_node.clone()), kind);
			}
		}
		_ => {}
	}
}

/// `xs = []; xs.push(5)`, lowered to `xs = xs + [5]`: the empty list takes the type of its first appended elements
pub(super) fn type_list_by_first_append(name: &str, value: &Node, scope: &mut Scope) {
	let Node::Key(list, Op::Add, appended) = value.drop_meta() else {
		return;
	};
	let appends_to_itself = list.is_symbol(name);
	if !appends_to_itself || !matches!(appended.drop_meta(), Node::List(_, Bracket::Square, _)) {
		return;
	}
	let appended_type = list_type_name(appended, scope);
	if let Some(local) =
		scope.own_binding_mut(name).filter(|local| local.kind == Kind::Empty && local.type_node.is_none())
	{
		local.kind = Kind::List;
		local.type_node = Some(Box::new(Node::Symbol(appended_type)));
	}
}

/// `m = {}; m[3] = 4`, lowered to `m = field_with(m, "3", 4)`: the empty value is a map from its first entry on
fn type_map_by_first_entry(name: &str, value: &Node, scope: &mut Scope) {
	if !crate::library_words::is_field_update_of(name, value) {
		return;
	}
	if let Some(local) = scope
		.own_binding_mut(name)
		.filter(|local| local.kind == Kind::Empty && local.type_node.is_none() && !local.is_param)
	{
		local.type_node = Some(Box::new(Node::Symbol(MAP_WORD.to_string())));
	}
}

/// `xs#i = v` with an element of another type than the inferred `list of <type>` of xs: a list of ints given a float
/// holds floats (`xs = [0, 0]; xs#1 = random()`), any other mix Nodes of any type; a declared type is kept (checked elsewhere)
pub(super) fn widen_element_type(scope: &mut Scope, list: &Node, value: &Node) {
	let Node::Symbol(name) = list.drop_meta() else { return };
	let assigned = element_type_word(value, scope);
	let Some(local) = scope.own_binding_mut(name) else {
		return;
	};
	let Some(element) = local
		.type_node
		.as_ref()
		.and_then(|type_node| type_node.name().strip_prefix(LIST_OF_PREFIX).map(str::to_string))
	else {
		return;
	};
	let widened = match (element.as_str(), assigned.as_str()) {
		(element, assigned) if element == assigned => return,
		(INT_WORD, FLOAT_WORD) => format!("{LIST_OF_PREFIX}{FLOAT_WORD}"),
		(_, INT_WORD) if element == RATIONAL_WORD || element == FLOAT_WORD => return, // an Int fits a rational or float list
		(FLOAT_WORD, RATIONAL_WORD) => return, // `xs#1 = 0.5` (an exact decimal) of a float list is stored as its f64
		_ => NODE_LIST_TYPE.to_string(),
	};
	set_widened_list_type(scope, name, widened);
}

/// The list `name` holds elements of the `widened` type, from its definition on once collected again (collect_variables)
fn set_widened_list_type(scope: &mut Scope, name: &str, widened: String) {
	let type_node = Box::new(Node::Symbol(widened));
	if let Some(local) = scope.own_binding_mut(name) {
		local.type_node = Some(type_node.clone());
	}
	scope.widened_bindings.insert(name.to_string(), (Kind::List, Some(type_node)));
}

/// `ys = ["a"]; ys = [3]`: a variable given lists of two element types holds the items of both, of their common type
/// (`list of number` for ints and floats), else held as Nodes; a declared type is kept (checked elsewhere)
fn widen_list_type(scope: &mut Scope, name: &str, value: &Node) {
	if infer_type(value, scope) != Kind::List {
		return;
	}
	let assigned = list_type_name(value, scope);
	let Some(local) = scope.own_binding_mut(name).filter(|local| local.kind == Kind::List && !local.is_param) else {
		return;
	};
	let Some(held) = local.type_node.as_ref().map(|type_node| type_node.name()) else {
		return;
	};
	let (Some(held_element), Some(assigned_element)) =
		(held.strip_prefix(LIST_OF_PREFIX), assigned.strip_prefix(LIST_OF_PREFIX))
	else {
		return;
	};
	if held_element == assigned_element {
		return;
	}
	let common = super::checks::common_type_word(&[held_element.to_string(), assigned_element.to_string()]);
	let widened = common.map_or(NODE_LIST_TYPE.to_string(), |element| format!("{LIST_OF_PREFIX}{element}"));
	set_widened_list_type(scope, name, widened);
}

/// An exact variable that is later assigned an f64 (`x=10; x=floor(2.5)` with libm's floor) holds an f64 throughout,
/// as an expression mixing in an f64 is one; a declared type is kept (and checked elsewhere). A global too: `b = 0.5`
/// changed by a function's `global b; b = b + random()`
pub(super) fn widen_to_float(scope: &mut Scope, name: &str, value: &Node) {
	let float_value = infer_type(value, scope).is_float();
	let is_local = scope.locals.contains_key(name);
	if let Some(local) =
		scope.own_binding_mut(name).filter(|local| local.kind == Kind::Int && local.type_node.is_none())
	{
		if float_value {
			local.kind = Kind::Float;
			// a read before this write (`y = x * 2` earlier in the loop) is a float too, once collected again
			if is_local {
				scope.widened_bindings.insert(name.to_string(), (Kind::Float, None));
			}
		}
	}
}

/// The kinds a variable may hold whose values are told apart only by representation; a variable given values of two
/// of them that do not mix (a list, then an Int; a text, then an Int) is a compile error when both kinds are evident
/// from the source (P45, check_kind_changes); where the compiler only infers them it is held as a Node of run-time kind.
/// Int and Float widen to Float instead (widen_to_float), a text and a character are both texts.
pub(super) const CONCRETE_KINDS: [Kind; 6] =
	[Kind::Int, Kind::Float, Kind::Text, Kind::Codepoint, Kind::List, Kind::Symbol];

pub(super) fn widen_to_node(scope: &mut Scope, name: &str, value: &Node) {
	let assigned = binding_kind(value, scope);
	// a parameter's representation comes from its calls (infer_parameters_from_calls), not from the body
	let Some(local) = scope
		.own_binding_mut(name)
		.filter(|local| !local.is_param && local.type_node.as_ref().is_none_or(|_| local.kind == Kind::List))
	else {
		return;
	};
	let mixes = |a: Kind, b: Kind| {
		a == b
			|| [a, b].iter().all(|kind| matches!(kind, Kind::Int | Kind::Float))
			|| [a, b].iter().all(|kind| matches!(kind, Kind::Text | Kind::Codepoint))
	};
	// a value of run-time kind may be anything: `for item in basket` after `item = 2`, `total = total + run.load`
	let run_time_value = crate::analyzer::inference::is_run_time_kind(&assigned) && !value.is_empty_node();
	let other_kind = run_time_value || CONCRETE_KINDS.contains(&assigned) && !mixes(local.kind, assigned);
	if CONCRETE_KINDS.contains(&local.kind) && other_kind {
		local.kind = Kind::Empty;
		local.type_node = None;
		// a read before this write (`x2 = x1` earlier in the loop) holds the Node too, once collected again
		if scope.locals.contains_key(name) {
			scope.widened_bindings.insert(name.to_string(), (Kind::Empty, None));
		}
	}
}

/// The kind of the `index`-th name of `x, y = …`: the tuple function's value kind, or that of the value in that position
pub(super) fn destructured_kind(values: &[Node], index: usize, scope: &Scope) -> Kind {
	match values {
		[call] => call
			.call_parts()
			.and_then(|(function, _)| scope.function_kinds.get(&crate::tuples::element_key(function, index)).copied())
			.unwrap_or(Kind::Data),
		_ => values.get(index).map_or(Kind::Data, |value| binding_kind(value, scope)),
	}
}

/// Kind of a new variable bound to `value`: `x=ø` makes x an optional, held as a Node that is ø until assigned
pub(super) fn binding_kind(value: &Node, scope: &Scope) -> Kind {
	held_kind(value, || infer_type(value, scope))
}

/// Kind of a variable or global holding `value`, whose expression kind is `inferred`
pub(crate) fn held_kind(value: &Node, inferred: impl FnOnce() -> Kind) -> Kind {
	match value.drop_meta() {
		Node::Empty => Kind::Empty,
		Node::List(items, Bracket::Curly, _) if items.is_empty() => Kind::Empty, // the empty block is ø as well
		Node::Char(_) => Kind::Text, // a variable holding "a" may later hold "ab": one-character texts are held as nodes
		// so does a copy of a character: `f(t) { n = t; n = prev[n] }` called with `f("F")`
		Node::Symbol(_) => match inferred() {
			Kind::Codepoint => Kind::Text,
			kind => kind,
		},
		_ => inferred(),
	}
}

/// Kind of a variable declared `x:T`; an optional `T?` and the empty type may hold ø, so they are held as a Node
pub(super) fn declared_kind(type_node: &Node) -> Option<Kind> {
	let type_name = type_node.drop_meta().to_string();
	let type_name = type_name.as_str();
	match type_name.strip_suffix('?') {
		Some(_) => Some(Kind::Empty),
		None if super::annotations::names_empty_type(type_node) => Some(Kind::Empty),
		// `y: any`: any value, held as a Node, never the kind of the first value given (card yes-stores)
		None if type_name == ANY_TYPE => Some(Kind::Empty),
		// an inline union `int or text` holds a value of either kind as a Node
		None if super::checks::union_parts(type_name).is_some() => Some(Kind::Empty),
		// `number` keeps whatever number it is given, an exact 1/3 or a float (card number-keeps-exact)
		None if crate::type_tests::holds_any_number(type_name) => Some(Kind::Empty),
		None => builtin_type_kind(type_name),
	}
}

/// Outer variables a function body reads: bound in `outer`, not a parameter or local of the body.
/// Functions capture these by value when they are defined (DESIGN.md: immutable local bindings).
pub fn captured_variables(function: &UserFunctionDef, outer: &Scope) -> Vec<(String, Kind)> {
	let mut own = Scope::new();
	for param in &function.params {
		own.define(param.name.clone(), None, param_kind(param));
	}
	collect_variables(&function.body, &mut own);
	for variable in crate::lowering::for_loop::loop_variables(&function.body) {
		own.define(variable, None, Kind::Empty);
	}
	let mut captured: Vec<(String, Kind)> = vec![];
	function.body.visit(&mut |node| {
		if let Node::Symbol(name) = node {
			let is_new = own.lookup(name).is_none() && !captured.iter().any(|(seen, _)| seen == name);
			if let Some(local) = outer.lookup(name).filter(|_| is_new) {
				captured.push((name.clone(), local.kind));
			}
		}
	});
	captured
}

/// Scope for tracking variable bindings
#[derive(Clone, Debug, Default)]
pub struct Scope {
	pub locals: HashMap<String, Local>,
	pub function_kinds: HashMap<String, Kind>, // Return kinds of user functions, which shadow FFI names like `pow`
	/// Variables that hold a closure → `closure_new` targets they may contain (per-site closure_call kinds)
	pub closure_variable_targets: HashMap<String, HashSet<String>>,
	pub globals: HashMap<String, Local>, // Declared `global` (kind and type, no slot): assigning them later must not create a shadowing local
	/// Locals a later write widened (an int to a float, a list's element type): their kind and type from the start
	/// (collect_variables)
	pub widened_bindings: HashMap<String, (Kind, Option<Box<Node>>)>,
	pub parent: Option<Box<Scope>>,
}

impl Scope {
	pub fn new() -> Self {
		Scope::default()
	}

	pub fn with_function_kinds(function_kinds: HashMap<String, Kind>) -> Self {
		Scope { function_kinds, ..Scope::default() }
	}

	pub fn with_closure_targets(mut self, closure_variable_targets: HashMap<String, HashSet<String>>) -> Self {
		self.closure_variable_targets = closure_variable_targets;
		self
	}

	/// Targets a closure variable may hold, looking through parent scopes
	pub fn closure_targets_of(&self, name: &str) -> Option<&HashSet<String>> {
		self.closure_variable_targets
			.get(name)
			.or_else(|| self.parent.as_ref().and_then(|parent| parent.closure_targets_of(name)))
	}

	pub fn is_global(&self, name: &str) -> bool {
		self.global(name).is_some()
	}

	pub fn global(&self, name: &str) -> Option<&Local> {
		self.globals.get(name).or_else(|| self.parent.as_ref().and_then(|parent| parent.global(name)))
	}

	/// The local or declared global a name refers to, for its kind and type
	pub fn binding(&self, name: &str) -> Option<&Local> {
		self.lookup(name).or_else(|| self.global(name))
	}

	/// A local or declared global of this scope itself, to refine its type
	pub fn own_binding_mut(&mut self, name: &str) -> Option<&mut Local> {
		match self.locals.contains_key(name) {
			true => self.locals.get_mut(name),
			false => self.globals.get_mut(name),
		}
	}

	pub fn function_kind(&self, name: &str) -> Option<Kind> {
		self.function_kinds.get(name).copied().or_else(|| self.parent.as_ref().and_then(|p| p.function_kind(name)))
	}

	/// A parameter here: it shadows a function of its name (`g(f, b) := [f, b]` lists f)
	pub fn is_parameter(&self, name: &str) -> bool {
		self.lookup(name).is_some_and(|local| local.is_param)
	}

	/// Look up a variable by name, checking parent scopes
	pub fn lookup(&self, name: &str) -> Option<&Local> {
		self.locals.get(name).or_else(|| self.parent.as_ref().and_then(|p| p.lookup(name)))
	}

	/// The names of the variables in this scope and its parents
	pub fn local_names(&self) -> Vec<String> {
		let mut names: Vec<String> = self.locals.keys().cloned().collect();
		if let Some(parent) = &self.parent {
			names.extend(parent.local_names());
		}
		names
	}

	/// Define a new variable in current scope
	pub fn define(&mut self, name: String, type_node: Option<Box<Node>>, kind: Kind) -> Local {
		let position = self.locals.len() as u32;
		let local = Local {
			name: name.clone(),
			type_node,
			declared: false,
			position,
			is_param: false,
			kind,
			data_pointer: 0,
			data_length: 0,
			may_be_ratio: false,
		};
		self.locals.insert(name, local.clone());
		local
	}

	/// Define a function parameter, with its declared type (`r: any`)
	pub fn define_param(&mut self, param: &crate::compiler::context::Param, kind: Kind) -> Local {
		let mut local = self.define(param.name.clone(), param.annotation.clone().map(Box::new), kind);
		local.is_param = true;
		local.declared = param.annotation.is_some();
		self.locals.insert(param.name.clone(), local.clone());
		local
	}

	/// Update a local's data pointer and length (for string assignments)
	pub fn set_local_data(&mut self, name: &str, pointer: u32, length: u32) {
		if let Some(local) = self.locals.get_mut(name) {
			local.data_pointer = pointer;
			local.data_length = length;
		}
	}

	/// Get total number of locals (for WASM local declaration)
	pub fn local_count(&self) -> u32 {
		self.locals.len() as u32
	}
}

pub(crate) fn is_constant_keyword(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if CONSTANT_KEYWORDS.contains(&word.as_str()))
}

pub(crate) fn is_declaration_keyword(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if is_declaration_word(word))
}

/// `var`, `let`, `const`, `val` …: a word declaring the name after it
pub(crate) fn is_declaration_word(word: &str) -> bool {
	CONSTANT_KEYWORDS.contains(&word) || word == VAR_KEYWORD || word == LET_WORD
}

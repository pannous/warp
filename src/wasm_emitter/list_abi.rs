//! The array calling convention for lists (notes/typed_lists.md): a function that works on a list parameter through its
//! array copy (`arr·list = arr`, analyzer::indexed_parameter_copies) takes that parameter as a `$NodeList` and copies it
//! with one array.copy, instead of a Node list the caller rebuilds and the callee converts back; a function whose every
//! result is such an array, or a call of another such function, returns the `$NodeList` itself. Callers convert only
//! where they need a Node: recursive list algorithms (quicksort_partitioned) stop converting at every call.

use super::list_dispatch::{ElementType, TypedList};
use super::WasmGcEmitter;
use crate::analyzer::{collect_variables, param_kind, Scope};
use crate::context::UserFunctionDef;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_kinds::Kind;
use std::collections::{HashMap, HashSet};
use wasm_encoder::*;
use Instruction as I;

const RETURN: &str = "return";

/// The list calling convention of one function
#[derive(Clone, Default, Debug)]
pub(super) struct ListAbi {
	/// Per parameter: passed as a `$NodeList`
	pub params: Vec<bool>,
	/// The result is a `$NodeList`
	pub returns_list: bool,
}

/// The variable each list parameter p is copied into, `v = p`, its only use: `p·list = p` (analyzer
/// indexed_parameter_copies) or a for loop's `items = p`
fn list_parameters(function: &UserFunctionDef) -> Vec<Option<String>> {
	function.params.iter().map(|param| {
		let mut copy = None;
		let mut uses = 0;
		function.body.visit(&mut |part| match part {
			Node::Symbol(name) if *name == param.name => uses += 1,
			Node::Key(target, Op::Assign, value) if matches!(value.drop_meta(), Node::Symbol(name) if *name == param.name) => {
				if let Node::Symbol(target) = target.drop_meta() {
					copy = Some(target.clone());
				}
			}
			_ => {}
		});
		// a text keeps its letters
		copy.filter(|_| uses == 1 && param.default.is_none() && param_kind(param) == Kind::List)
	}).collect()
}

/// What a body gives back: its last statement (or what its final `return` returns) and every `return` value
fn results(body: &Node) -> Vec<&Node> {
	let mut found = vec![];
	body.visit(&mut |part| if let Node::List(items, _, _) = part {
		if let [word, value] = items.as_slice() {
			if matches!(word.drop_meta(), Node::Symbol(name) if name == RETURN) {
				found.push(value);
			}
		}
	});
	let last = match body.drop_meta() {
		Node::List(statements, Bracket::Curly, _) => statements.last(),
		_ => Some(body), // `f(x) := (…; v)` is worth v (last_value)
	};
	if let Some(last) = last {
		let returns = matches!(last.drop_meta(), Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if name == RETURN));
		if !returns {
			found.push(last);
		}
	}
	found
}

/// The last value of a block `(…; x)`
fn last_value(node: &Node) -> &Node {
	match node.drop_meta() {
		Node::List(items, Bracket::Round | Bracket::Curly, Separator::Semicolon | Separator::Newline) if !items.is_empty() => last_value(&items[items.len() - 1]),
		other => other,
	}
}

/// The function a call `f(…)` calls
pub(super) fn called_function(node: &Node) -> Option<&str> {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, Separator::None) => match items.first().map(Node::drop_meta) {
			Some(Node::Symbol(name)) => Some(name),
			_ => None,
		},
		_ => None,
	}
}

impl WasmGcEmitter {
	/// The list convention of every user function that has one, decided before the signatures are registered
	pub(super) fn find_list_abi(&mut self) -> HashMap<String, ListAbi> {
		let excluded: HashSet<String> = self.ctx.closure_targets.iter().map(|(target, _)| target.clone()).collect();
		let functions: Vec<UserFunctionDef> = self.ctx.user_functions.values()
			.filter(|function| !excluded.contains(&function.name) && function.tuple_kinds.is_empty() && !self.is_closure_call(&function.name))
			.cloned().collect();
		let mut abi: HashMap<String, ListAbi> = HashMap::new();
		let mut list_results: HashMap<String, Vec<Node>> = HashMap::new();
		for function in &functions {
			let copies = list_parameters(function);
			let mut params: Vec<bool> = copies.iter().map(Option::is_some).collect();
			if !params.contains(&true) {
				continue;
			}
			// a parameter whose copy is no array after all takes a Node
			let mut typed = self.body_typed_lists(function, &params);
			let arrays: Vec<bool> = copies.iter().map(|copy| copy.as_ref().is_some_and(|copy| typed.get(copy).is_some_and(|list| list.element == ElementType::Node))).collect();
			if arrays != params {
				params = arrays;
				if !params.contains(&true) {
					continue;
				}
				typed = self.body_typed_lists(function, &params);
			}
			// a result that is the array copy itself, held as a $NodeList; the calls among the results are judged below
			let results: Vec<Node> = results(&function.body).into_iter().map(|result| last_value(result).clone()).collect();
			let arrays = results.iter().all(|result| match result.drop_meta() {
				Node::Symbol(name) => typed.get(name).is_some_and(|list| list.element == ElementType::Node),
				other => called_function(other).is_some(),
			});
			if arrays {
				list_results.insert(function.name.clone(), results);
			}
			abi.insert(function.name.clone(), ListAbi { params, returns_list: false });
		}
		// greatest fixpoint: a result call returns an array when its function does
		let mut returning: HashSet<String> = list_results.keys().cloned().collect();
		loop {
			let kept: HashSet<String> = returning.iter().filter(|name| list_results[*name].iter().all(|result| match called_function(result) {
				Some(callee) => returning.contains(callee),
				None => true,
			})).cloned().collect();
			if kept == returning {
				break;
			}
			returning = kept;
		}
		for name in returning {
			abi.get_mut(&name).expect("a list function").returns_list = true;
		}
		abi
	}

	/// The typed lists of a function body, as compile_user_function_body will find them, its list parameters typed
	pub(super) fn body_typed_lists(&mut self, function: &UserFunctionDef, list_params: &[bool]) -> HashMap<String, TypedList> {
		let scope = Scope::with_function_kinds(self.user_function_kinds()).with_closure_targets(self.ctx.closure_variable_targets.clone());
		let saved_scope = std::mem::replace(&mut self.scope, scope);
		self.scope.globals = self.ctx.declared_globals.clone();
		for (param, is_list) in function.params.iter().zip(list_params) {
			self.scope.define_param(param.name.clone(), if *is_list { Kind::List } else { param_kind(param) });
		}
		collect_variables(&function.body, &mut self.scope);
		let mut typed = self.find_typed_lists(&function.body);
		self.scope = saved_scope;
		for (param, _) in function.params.iter().zip(list_params).filter(|(_, is_list)| **is_list) {
			typed.insert(param.name.clone(), TypedList { element: ElementType::Node, updated: true });
		}
		typed
	}

	pub(super) fn node_list_type(&self) -> ValType {
		ValType::Ref(RefType { nullable: true, heap_type: HeapType::Concrete(self.type_manager.node_list_type) })
	}

	/// Does a call of this function give back a $NodeList
	pub(super) fn returns_list_abi(&self, name: &str) -> bool {
		self.list_abi.get(name).is_some_and(|abi| abi.returns_list)
	}

	pub(super) fn takes_list_abi(&self, name: &str, index: usize) -> bool {
		self.list_abi.get(name).is_some_and(|abi| abi.params.get(index).copied().unwrap_or(false))
	}

	/// A value where a $NodeList is wanted: a Node-list variable as its array, a call of a function returning one as it
	/// is, any other Node list converted once
	pub(super) fn emit_list_abi_value(&mut self, func: &mut Function, value: &Node) {
		// `(statements; x)`: the statements run, x is the value
		if let Node::List(items, Bracket::Round | Bracket::Curly, Separator::Semicolon | Separator::Newline) = value.drop_meta() {
			if let Some((last, statements)) = items.split_last() {
				self.emit_discarded_statements(func, statements);
				return self.emit_list_abi_value(func, last);
			}
		}
		if let Node::Symbol(name) = value.drop_meta() {
			if self.typed_lists.get(name).is_some_and(|list| list.element == ElementType::Node) {
				let slot = self.scope.lookup(name).expect("a typed list is a local").position;
				func.instruction(&I::LocalGet(slot));
				return;
			}
			if self.emit_typed_list_as_node_list(func, name) {
				return;
			}
		}
		if let Some(callee) = called_function(value).filter(|callee| self.returns_list_abi(callee)) {
			let callee = callee.to_string();
			let Node::List(items, _, _) = value.drop_meta() else { unreachable!("a call") };
			let user_fn = self.ctx.user_functions[&callee].clone();
			self.emit_user_function_call_inner(func, &user_fn, &items[1..]);
			return;
		}
		self.emit_node_instructions(func, value);
		self.emit_call(func, "node_list_of");
	}
}

impl WasmGcEmitter {
	/// The body of a function returning a $NodeList: its statements, then its last value as the array
	pub(super) fn emit_list_abi_body(&mut self, func: &mut Function, body: &Node) {
		let statements = match body.drop_meta() {
			Node::List(items, Bracket::Curly, _) => items.clone(),
			other => vec![other.clone()],
		};
		let Some((last, before)) = statements.split_last() else {
			self.emit_list_abi_value(func, &Node::Empty);
			return;
		};
		for statement in before {
			if self.is_definition(statement) {
				continue;
			}
			if !self.emit_loop_jump(func, statement) {
				self.emit_discarded_statement(func, statement, Self::emit_node_instructions);
				func.instruction(&I::Drop);
			}
		}
		let is_return = matches!(last.drop_meta(), Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if name == RETURN));
		if is_return {
			self.emit_node_instructions(func, last); // returns, leaving nothing reachable
		} else {
			self.emit_list_abi_value(func, last);
		}
	}
}

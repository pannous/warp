//! Pipes: `xs | f`, `f.(xs)` broadcast calls and pipe parameters
use super::expressions::BODY_ENDS;
use super::*;

/// `a | b`: the logical or, or `b(a)` when b names a function (pipes.rs)
const PIPE_GLYPH: &str = "|";

/// `f.(xs, a)` → `map(xs, broadcast_item => f(broadcast_item, a))`
pub(super) fn dot_call(function: Node, arguments: Node) -> Node {
	let mut arguments = match arguments.drop_meta() {
		Node::List(items, Bracket::Round, _) if !items.is_empty() => items.clone(),
		_ => vec![arguments],
	};
	// a number broadcasts as itself (Julia, Elixir's `f.(4)`): the call
	if matches!(arguments[0].drop_meta(), Node::Number(_)) {
		return Node::List([vec![function], arguments].concat(), Bracket::Round, Separator::None);
	}
	let list = arguments.remove(0);
	let item = Symbol(BROADCAST_ITEM.to_string());
	let applied = Node::List([vec![function, item.clone()], arguments].concat(), Bracket::Round, Separator::None);
	let each = Node::Key(Box::new(item), Op::FatArrow, Box::new(applied));
	call(MAP_WORD, vec![list, each])
}

/// `value |> f(args)` → `f(value, args)`, `value |> f` → `f(value)`; a braceless call is grouped, one argument:
/// `square xs |> filter(p)` → `filter((square xs), p)`
pub(crate) fn piped(value: Node, stage: Node) -> Node {
	let value = match value.drop_meta() {
		Node::List(_, Bracket::None, Separator::Space) => Node::List(vec![value], Bracket::Round, Separator::None),
		_ => value,
	};
	match stage.drop_meta() {
		Node::List(items, Bracket::Round, Separator::None)
			if matches!(items.first().map(Node::drop_meta), Some(Symbol(_))) =>
		{
			let mut items = items.clone();
			items.insert(1, value);
			Node::List(items, Bracket::Round, Separator::None)
		}
		_ => Node::List(vec![stage, value], Bracket::Round, Separator::None),
	}
}

impl WarpParser {
	/// The right operand of a special infix operator, None when it binds looser than `min_bp` (nothing consumed)
	pub(super) fn at_pipeline(&self) -> bool {
		self.current_char() == '|' && self.peek_char(1) == '>'
	}

	/// `|x| x*x`, `|a, b| a+b` (Rust, and Ruby's `{ |x| x*x }`): the parameters of a lambda between bars, which the
	/// body follows; nothing else starts with a bar
	pub(super) fn pipe_parameters(&mut self) -> Option<Node> {
		if self.current_char() != '|' || self.options.data_mode {
			return None;
		}
		let mut length = 1;
		while matches!(self.peek_char(length), ' ' | ',')
			|| self.peek_char(length).is_alphanumeric()
			|| self.peek_char(length) == '_'
		{
			length += 1;
		}
		let written: String = (1..length).map(|offset| self.peek_char(offset)).collect();
		let names: Vec<Node> = written
			.split([' ', ','])
			.filter(|name| !name.is_empty())
			.map(|name| Node::Symbol(name.to_string()))
			.collect();
		let mut body_start = length + 1;
		while self.peek_char(body_start) == ' ' {
			body_start += 1;
		}
		let first_name = 1 + written.len() - written.trim_start().len();
		if self.peek_char(length) != '|'
			|| names.is_empty()
			|| !self.is_identifier_start(first_name)
			|| BODY_ENDS.contains(&self.peek_char(body_start))
		{
			return None;
		}
		self.advance_by(body_start);
		Some(match <[Node; 1]>::try_from(names) {
			Ok([name]) => name,
			Err(names) => Node::List(names, Bracket::Round, Separator::Comma),
		})
	}

	/// The operand after a single `|`: a bare word or word operator is marked as a possible pipe stage, any other operand gets the hint
	/// toward `or`
	pub(super) fn pipe_operand(&self, operand: Node, line: usize, column: usize) -> Node {
		if crate::pipes::may_be_stage(&operand) {
			return crate::pipes::pipe_stage(operand);
		}
		set_hint_position(line, column);
		norm::operator(PIPE_GLYPH, false);
		operand
	}
}

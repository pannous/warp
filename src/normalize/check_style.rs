//! Style check of a parsed program: the forms that are only visible in the AST
use super::*;

/// The acknowledge-once note that `let` is immutable in warp
pub const LET_TOPIC: &str = "let";
/// The acknowledge-once note on `x ? y : z` against `if x then y else z`
pub const CONDITIONAL_TOPIC: &str = "conditional";

/// The quote character of the canonical string style
pub fn text_quote() -> char {
	match style().quotes {
		QuoteStyle::Single => '\'',
		QuoteStyle::Double | QuoteStyle::Any => '"',
	}
}

/// Source text of an operand for a hint
pub fn operand_text(node: &Node) -> String {
	match node.drop_meta() {
		Node::Text(text) => format!("{0}{text}{0}", text_quote()),
		other => as_written(other.clone()).serialize(),
	}
}

/// The node as its source spelled it where a pass lowered sugar: `floor_quotient(a, b)` is `a /! b`, `area·square` is `area`
fn as_written(node: Node) -> Node {
	match node {
		Node::List(ref items, _, _)
			if let [word, dividend, divisor] = items.as_slice()
				&& matches!(word.drop_meta(), Node::Symbol(name) if name == crate::warp_parser::FLOOR_QUOTIENT) =>
		{
			Node::Symbol(format!("{} /! {}", operand_text(dividend), operand_text(divisor)))
		}
		// a witness `area·square` was written `area`
		Node::Symbol(name) if name.contains('·') => {
			Node::Symbol(name.split('·').next().unwrap_or_default().to_string())
		}
		other => crate::lowering::nodes::children_rewritten(other, as_written),
	}
}

/// Visit every node as `(node without metadata, node with metadata)`: the second one knows the source position
fn walk<'a>(node: &'a Node, action: &mut dyn FnMut(&'a Node, &'a Node)) {
	let bare = node.drop_meta();
	action(bare, node);
	match bare {
		Node::Key(left, _, right) => {
			walk(left, action);
			walk(right, action);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| walk(item, action)),
		_ => {}
	}
}

/// The position where the node starts: its own, else that of its leftmost child; unknown positions are cleared, never stale
pub fn set_position_of(node: &Node) {
	fn start_of(node: &Node) -> Option<crate::compiler::meta::LineInfo> {
		if let Some(info) = node.get_lineinfo() {
			return Some(info);
		}
		match node {
			Node::Meta { node, .. } => start_of(node),
			// a word a pass made (`times` of `3 times 4`) has no position: the first part that has one
			Node::Key(left, _, right) => start_of(left).or_else(|| start_of(right)),
			Node::List(items, _, _) => items.iter().find_map(start_of),
			_ => None,
		}
	}
	match start_of(node) {
		Some(position) => set_hint_position_in(position.file, position.line_nr, position.column),
		None => clear_hint_position(),
	}
}

/// `x:list of int=[1 2]` is the items `x:list`, `of`, `int=[1 2]`: the element words after `of`, and the node of the head word `list`
fn of_list_type(items: &[Node], of_index: usize) -> Option<(&Node, Vec<&str>)> {
	let Node::Key(_, Op::Colon, head) = items.get(of_index.checked_sub(1)?)?.drop_meta() else {
		return None;
	};
	(crate::warp_parser::list_type_head(head.symbol_name()?) == LIST_WORD).then_some(())?;
	let mut words = Vec::new();
	for item in &items[of_index + 1..] {
		match item.drop_meta() {
			Node::Symbol(word) if word == OF_WORD => continue,
			Node::Symbol(word) => words.push(word.as_str()),
			Node::Key(word, _, _) => {
				words.push(word.symbol_name()?);
				break;
			}
			_ => return None,
		}
	}
	(!words.is_empty()).then_some((head, words))
}

/// The name and parameter list of a definition after its keyword: `def (f x):…`, `fn (f x)=…`, `function ((f (x)) {…})`
fn signature_of(definition: &Node) -> Option<(String, String, BodyForm)> {
	let (signature, body_form) = match definition.drop_meta() {
		Node::Key(signature, Op::Colon, _) => (signature.as_ref(), BodyForm::Colon),
		Node::Key(signature, _, _) => (signature.as_ref(), BodyForm::Assign),
		Node::List(parts, _, _) => (parts.first()?, BodyForm::Block),
		_ => return None,
	};
	let Node::List(words, _, _) = signature.drop_meta() else {
		return None;
	};
	let (name, parameters) = words.split_first()?;
	let parameters: Vec<String> = parameters
		.iter()
		.map(|parameter| parameter.serialize().trim_matches(|c| c == '(' || c == ')').to_string())
		.collect();
	Some((name.symbol_name()?.to_string(), parameters.join(", "), body_form))
}

/// Emit the hints for every non-canonical form in the parsed program that the parser cannot see while reading characters
pub fn check_style(program: &Node) {
	walk(program, &mut |node, positioned| match node {
		// `[int, 3]` and `[double, f]` are items, never the call int(3)
		Node::List(items, bracket, separator)
			if *bracket != Bracket::Square && *separator != crate::node::Separator::Comma =>
		{
			check_items(items, positioned)
		}
		Node::Key(_, Op::Colon, right) => {
			if let Node::List(items, Bracket::Square, _) = right.drop_meta() {
				if let [element] = items.as_slice() {
					if let Some(word) = element.symbol_name() {
						set_position_of(right);
						hints::list_type(ListTypeStyle::Bracket, &[word]);
					}
				}
				return;
			}
			let Some(type_word) = right.symbol_name() else { return };
			set_position_of(right);
			if let Some(element) = crate::analyzer::plural_element_type(type_word) {
				hints::list_type(ListTypeStyle::Plural, &[element]);
			}
			hints::string_type(type_word);
		}
		Node::Key(value, Op::As, type_node) => {
			if let Some(type_word) = type_node.symbol_name() {
				set_position_of(positioned);
				if !hints::as_operator(&operand_text(value), type_word) {
					hints::string_type(type_word);
				}
			}
		}
		Node::Key(target, Op::Define, value) if target.symbol_name().is_some() => {
			set_position_of(positioned);
			hints::var_keyword(":=", &operand_text(target), &operand_text(value));
		}
		Node::Key(_, Op::Define, _) => {
			if let Some((name, parameters, _)) = signature_of(node) {
				set_position_of(positioned);
				hints::function_keyword(":=", &name, &parameters, BodyForm::Assign);
			}
		}
		Node::Key(target, Op::Dot, call) => {
			if matches!(call.drop_meta(), Node::List(words, _, _) if words.len() == 1 && (words[0]).symbol_name() == Some("length"))
			{
				set_position_of(positioned);
				hints::length_operator(&operand_text(target), true);
			}
		}
		_ => {}
	});
}

fn check_items(items: &[Node], positioned: &Node) {
	if let Some(of_index) = items.iter().position(|item| item.symbol_name() == Some(OF_WORD)) {
		if let Some((head, words)) = of_list_type(items, of_index) {
			set_position_of(head);
			let _ = crate::warp_parser::noted_list_type_head(head.symbol_name().unwrap_or_default());
			hints::list_type(ListTypeStyle::Of, &words);
		}
	}
	let [keyword, rest @ ..] = items else { return };
	let Some(keyword_word) = keyword.symbol_name() else {
		return;
	};
	set_position_of(positioned);
	match (keyword_word, rest) {
		("let" | "var", [declaration, ..]) => {
			if let Node::Key(name, Op::Assign | Op::Define, value) = declaration.drop_meta() {
				hints::var_keyword(keyword_word, &operand_text(name), &operand_text(value));
			}
		}
		("len", [argument]) => {
			let counted = operand_text(argument);
			let counted = counted.trim_matches(|c| c == '(' || c == ')');
			hint(&format!("len({counted})"), &format!("#{counted}"), "use # prefix for length");
		}
		(word, [definition, ..]) if crate::operators::is_function_keyword(word) => {
			if let Some((name, parameters, body_form)) = signature_of(definition) {
				hints::function_keyword(word, &name, &parameters, body_form);
			}
		}
		// `codepoint(c)` is a library word (P74), not the type's constructor
		(type_word, [argument])
			if crate::analyzer::type_word_kind(&type_word.to_lowercase()).is_some()
				&& !crate::library_words::is_library_word(type_word) =>
		{
			let is_call = match argument.drop_meta() {
				// `int g(int x) {…}` defines g, `int(…)` would convert
				Node::List(parts, Bracket::Round, _) if matches!(parts.as_slice(), [_, body] if matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _))) => {
					false
				}
				// `text(p:person)` heads the Printable operation of a declared type (P31)
				Node::List(parts, Bracket::Round, _) if matches!(parts.as_slice(), [part] if matches!(part.drop_meta(), Node::Key(_, Op::Colon, _))) => {
					false
				}
				Node::Key(_, Op::Colon, _) => false,
				Node::List(_, Bracket::Round, _) => true,
				Node::Symbol(_) | Node::Key(_, Op::Assign | Op::Define, _) => false, // `int x` and `int x = 1` declare
				_ => true,
			};
			if is_call {
				let value = match argument.drop_meta() {
					Node::List(inner, Bracket::Round, _) if inner.len() == 1 => &inner[0],
					other => other,
				};
				if !hints::type_constructor(type_word, &operand_text(value)) {
					hints::string_type(type_word);
				}
			}
		}
		_ => {}
	}
}

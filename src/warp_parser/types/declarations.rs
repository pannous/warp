//! Type declarations: `class`, `struct`, `type`, `record` with their bodies, conformances, primary constructors, type parameters, sum types and enums with values
use super::*;
use crate::words::ANY_TYPE;

/// Swift's `enum Shape { case circle(radius: Double) }`
const CASE_KEYWORD: &str = "case";
/// Python's root class `class Point(object):`, no parent of its own
const PYTHON_ROOT_CLASS: &str = "object";
/// The field of a sum type's variant with one unnamed payload, `Some(T)`: `Some(3).value`; several are value1, value2…
const VARIANT_FIELD: &str = "value";

/// What follows the name of a lowercase type declaration: `type point {…}`, `type size = u32`, `type pair<T>`
pub(crate) const TYPE_BODY_STARTS: [char; 4] = ['{', '=', ':', '<'];

impl WarpParser {
	/// `class Name {…}`, `struct`, `type Name {…}`, `record Name {…}`; `class:"btn"`, `type:email` (html attributes),
	/// `class="btn"`, `type = "text"` (html attributes, P188), `x.class`, `type(x)` and `type 3.5` are no declarations
	pub(in crate::warp_parser) fn declares_type(&self, symbol: &str) -> bool {
		let blanks = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		let is_assigned = self.peek_char(blanks) == '=' && self.peek_char(blanks + 1) != '=';
		let is_key = (self.current_char() == ':' && self.peek_char(1) != '=') || is_assigned;
		let start = self.pos.saturating_sub(symbol.chars().count());
		let is_field = start > 0 && self.chars[start - 1] == '.';
		!is_key
			&& !is_field
			&& (TYPE_DECLARATION_WORDS.contains(&symbol)
			|| (symbol == RECORD_WORD && (self.name_and_block_follow() || self.name_and_parameters_follow()))
			// `type 3.5`, `type pi`, `type(x)`: the type of a value; a declaration names its type
			|| (symbol == "type" && self.type_declaration_follows()))
	}

	/// After `type`: a capitalized name (`type Point`) or any name with its body (`type point {…}`, `type size = u32`)
	pub(crate) fn type_declaration_follows(&self) -> bool {
		if !self.name_after_blanks() {
			return false;
		}
		let blanks = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		let name_length = (blanks..)
			.take_while(|&offset| is_identifier_char(self.peek_char(offset)) || self.peek_char(offset) == '-')
			.count();
		let after_name = (blanks + name_length..)
			.find(|&offset| !matches!(self.peek_char(offset), ' ' | '\t'))
			.unwrap_or(blanks + name_length);
		let body_follows =
			TYPE_BODY_STARTS.contains(&self.peek_char(after_name)) && self.peek_char(after_name + 1) != '=';
		// `type of x` arrives as the declaration of `of`, which lowering (type_tests::type_of_word) reads as `type(x)`
		let of_follows = (blanks..blanks + name_length).map(|offset| self.peek_char(offset)).eq("of".chars());
		self.peek_char(blanks).is_uppercase() || body_follows || of_follows
	}

	/// The name and the `{fields}` of a type declaration, after its keyword
	pub(in crate::warp_parser) fn parse_type_declaration(&mut self) -> Node {
		let outer = self.type_fields.replace(Default::default());
		let declaration = self.parse_type_declaration_body();
		self.type_fields = outer;
		declaration
	}

	/// `class Foo` as written, its name marked when no body follows (BARE_CLASS_MARK: class_references.rs reads it as the
	/// type Foo unless the program declares Foo so); `class Foo {}`, `sealed class Foo` and `type Foo` stay declarations
	pub(in crate::warp_parser) fn parse_class_declaration(&mut self) -> Node {
		let before = self.mark();
		let declaration = self.parse_type_declaration();
		let written = self.chars[before.pos..self.pos].iter().collect::<String>();
		match declaration {
			Node::Type { name, body } if !written.contains(['{', ':']) => {
				Node::Type { name: Box::new(name.with_attribute(crate::words::BARE_CLASS_MARK, Node::True)), body }
			}
			other => other,
		}
	}

	pub(crate) fn parse_type_declaration_body(&mut self) -> Node {
		self.skip_whitespace();
		let type_name = or_return_error!(self.parse_symbol());
		let type_name = self.joined_type_name(type_name);
		// `class Box<T>{item:T}`: a field or parameter of a type parameter holds any value
		// `type Option[T] = Some(T) | None` (samples/types/types.warp) is warp's own spelling; P157: warp has no generic syntax, a
		// ported `class Box<T>` compiles untyped, with a note
		let angled = self.current_char() == '<';
		let type_parameters = match self.current_char() {
			'<' | '[' => or_return_error!(self.parse_type_parameters()),
			_ => vec![],
		};
		if angled {
			crate::normalize::hint(
				&format!("{type_name}<{}>", type_parameters.join(", ")),
				&type_name,
				"warp infers types: write the class without type parameters",
			);
		}
		if let Some(variants_start) = self.variants_ahead() {
			return self.parse_sum_type(type_name, &type_parameters, variants_start);
		}
		let mut constructor_fields = if self.current_char() == '(' { self.parse_primary_constructor() } else { vec![] };
		let (kotlin_parent, claims) = self.skip_conformances().unwrap_or_default();
		// Python's `class Dog(Animal):` names its parents in the parentheses, `object` the root of all
		let python_body = self.current_char() == ':' && self.peek_char(1) != '=';
		let python_parent = python_body
			.then(|| {
				std::mem::take(&mut constructor_fields)
					.into_iter()
					.map(|parent| parent.drop_meta().name())
					.find(|parent| parent != PYTHON_ROOT_CLASS)
			})
			.flatten();
		let ends_statement = matches!(
			self.peek_char((0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count()),
			'\n' | ';' | '}' | '\0'
		);
		let before_body = self.mark();
		// `class P(val x: Int)` ends at its line when no body follows on it
		if constructor_fields.is_empty() {
			self.skip_whitespace();
		} else {
			self.skip_blanks();
		}
		// `class dog extends animal {…}` (P117): the parent rides on the name, class_methods copies its fields and methods
		let mut name = Symbol(type_name);
		if let Some(parent) = python_parent.or(kotlin_parent) {
			name = name.with_attribute(EXTENDS_KEYWORD, Symbol(parent));
		}
		// P177: `implements Shape`, Swift's `: Shape` claim the traits, checked like `class Square{…} is Shape`
		if !claims.is_empty() {
			name = name.with_attribute(IMPLEMENTS_WORD, Node::List(claims, Bracket::None, Separator::Space));
		}
		if self.matches_keyword(EXTENDS_KEYWORD) {
			self.advance_by(EXTENDS_KEYWORD.len());
			self.skip_whitespace();
			match self.parse_symbol() {
				Ok(parent) => name = name.with_attribute(EXTENDS_KEYWORD, Symbol(parent)),
				Err(message) => return error(&message),
			}
			self.skip_whitespace();
		}
		// `class Duck with Walker, Swimmer {…}`: the mixins ride on the name, class_methods takes in their items
		if self.matches_keyword(WITH_KEYWORD) {
			self.advance_by(WITH_KEYWORD.len());
			let mixins = or_return_error!(self.parse_comma_names());
			name = name.with_attribute(WITH_KEYWORD, Node::List(mixins, Bracket::None, Separator::Space));
		}
		// Go's `type Shape interface {…}`: the trait Shape, as `interface Shape {…}` declares it (traits.rs)
		if self.matches_keyword(GO_INTERFACE_WORD) {
			self.advance_by(GO_INTERFACE_WORD.len());
			self.skip_whitespace();
			if self.current_char() == '{' {
				let block = self.parse_bracketed('{');
				return Node::List(
					vec![Symbol(GO_INTERFACE_WORD.to_string()), name, block],
					Bracket::None,
					Separator::Space,
				);
			}
		}
		self.skip_struct_words();
		let body = if self.current_char() == '{' { Self::class_body(self.parse_bracketed('{')) } else { Empty };
		// Python's `class Point:` and the lines indented below it
		let body = match (python_body, body) {
			(true, Empty) => {
				self.advance(); // :
				self.skip_spaces();
				self.skip_struct_words();
				// `type Point: {` and its fields on the lines below: the braces are the body, as in `type Point {`
				if self.current_char() == '{' {
					Self::class_body(self.parse_bracketed('{'))
				} else {
					self.parse_indented_block().map(Self::transform_fields_to_types).unwrap_or(Empty)
				}
			}
			// Ruby's `class Point` and the lines indented below it, up to its `end`
			(false, Empty) if self.pos > before_body.pos && self.closing_end_follows(&RUBY_END_OPENERS) => {
				self.rewind(before_body);
				let body = self
					.parse_indented_block()
					.map(without_end_lines)
					.map(Self::transform_fields_to_types)
					.unwrap_or(Empty);
				self.skip_whitespace();
				if self.matches_keyword(END_KEYWORD) {
					self.advance_by(END_KEYWORD.len());
				}
				body
			}
			(_, body) => body,
		};
		let body = if constructor_fields.is_empty() {
			body
		} else {
			let fields = constructor_fields.into_iter().map(Self::transform_fields_to_types);
			Node::List(fields.chain(statements(body)).collect(), Bracket::Curly, Separator::Semicolon)
		};
		let body = if type_parameters.is_empty() { body } else { any_for_type_parameters(body, &type_parameters) };
		// Kotlin's `sealed class Shape`, Swift's `class Marker`: a class without fields, when its line ends after the name
		// (`type of x` goes on: no declaration)
		let bodyless = matches!(body, Empty) && ends_statement;
		let body = if bodyless { Node::List(vec![], Bracket::Curly, Separator::None) } else { body };
		// C#'s file-scoped `class Foo;` takes in the definitions after it (file_declarations.rs)
		if bodyless && self.current_char() == ';' {
			name = name.with_attribute(crate::words::FILE_CLASS_MARK, Node::True);
		}
		Node::Type { name: Box::new(name), body: Box::new(body) }
	}

	/// `Walker, Swimmer`: names separated by commas
	pub(crate) fn parse_comma_names(&mut self) -> Result<Vec<Node>, String> {
		let mut names = vec![];
		loop {
			self.skip_whitespace();
			names.push(Symbol(self.parse_symbol()?));
			self.skip_whitespace();
			if self.current_char() != ',' {
				return Ok(names);
			}
			self.advance();
		}
	}

	/// Go's `type Point struct {…}`, WebAssembly's `type Node: gc struct {…}`: the words before the fields
	pub(crate) fn skip_struct_words(&mut self) {
		for word in [GC_WORD, GO_STRUCT_WORD] {
			if self.matches_keyword(word) {
				self.advance_by(word.len());
				self.skip_spaces();
			}
		}
	}

	/// The traits a class names before its body, Java's and TypeScript's `implements Shape, Named {`, Swift's
	/// `struct Square: Shape {`, Kotlin's `class Square(…) : Shape {`: the parent and the claimed traits. A type conforms
	/// to a trait by defining its operations (notes/traits.md T2); a named one is a claim, checked (P177). Kotlin's
	/// superclass `: Shape()` (its constructor called) is the parent, as `extends Shape`
	pub(crate) fn skip_conformances(&mut self) -> Option<(Option<String>, Vec<Node>)> {
		let blanks = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		let start = if self.peek_char(blanks) == ':' && self.peek_char(blanks + 1) != '=' {
			blanks + 1
		} else if (0..IMPLEMENTS_WORD.len())
			.all(|i| self.peek_char(blanks + i) == IMPLEMENTS_WORD.as_bytes()[i] as char)
			&& !is_identifier_char(self.peek_char(blanks + IMPLEMENTS_WORD.len()))
		{
			blanks + IMPLEMENTS_WORD.len()
		} else {
			return None;
		};
		let names: String =
			(start..).map(|offset| self.peek_char(offset)).take_while(|ch| !matches!(ch, '{' | '\n' | '\0')).collect();
		let names_list: Vec<&str> = names.split(',').map(str::trim).collect();
		let is_name = |name: &str| !name.is_empty() && name.chars().all(is_identifier_char);
		let parent = names_list.iter().find_map(|name| name.strip_suffix("()").filter(|parent| is_name(parent)));
		let are_names = names_list.iter().all(|name| is_name(name.strip_suffix("()").unwrap_or(name)));
		let ends_well = self.peek_char(start + names.chars().count()) == '{' || parent.is_some();
		if !are_names || !ends_well {
			return None;
		}
		let parent = parent.map(str::to_string);
		let claims =
			names_list.iter().filter(|name| !name.ends_with("()")).map(|name| Symbol(name.to_string())).collect();
		self.advance_by(start + names.trim_end().chars().count());
		Some((parent, claims))
	}

	/// The name of a declared type after blanks: `new Point(…)`
	pub(crate) fn declared_type_after_blanks(&self) -> bool {
		let blanks = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		let name: String =
			(blanks..).map(|offset| self.peek_char(offset)).take_while(|ch| is_identifier_char(*ch)).collect();
		blanks > 0 && self.declared_types.contains(&name)
	}

	/// `class` or `struct` after blanks, the word a class modifier stands before
	pub(crate) fn class_keyword_after_blanks(&self) -> Option<&'static str> {
		let blanks = self.blanks_ahead();
		TYPE_DECLARATION_WORDS.into_iter().find(|keyword| {
			keyword.chars().enumerate().all(|(i, c)| self.peek_char(blanks + i) == c)
				&& !is_identifier_char(self.peek_char(blanks + keyword.len()))
		})
	}

	/// `constant x`: a name after one or more blanks
	pub(crate) fn identifier_after_blanks(&self) -> bool {
		let blanks = self.blanks_ahead();
		blanks > 0 && (self.peek_char(blanks).is_alphabetic() || self.peek_char(blanks) == '_')
	}

	/// Kotlin's primary constructor `class Point(val x: Int, var y: Int = 0)`: its parameters are the fields
	pub(crate) fn parse_primary_constructor(&mut self) -> Vec<Node> {
		let parameters = statements(self.parse_bracketed('('));
		parameters
			.into_iter()
			.map(|parameter| match parameter.drop_meta() {
				// `val x: Int`, `var y: Int = 0`: the field, its keyword dropped
				Node::List(words, _, _)
					if words.len() == 2 && FIELD_KEYWORDS.contains(&words[0].drop_meta().name().as_str()) =>
				{
					let field = crate::node::leading_name(&words[1]);
					crate::errors::diagnostic::note_alias(&format!("{} {field}", words[0].drop_meta().name()), &field);
					words[1].clone()
				}
				// C#'s `int X`: the field X of type int
				Node::List(words, _, _)
					if words.len() == 2
						&& matches!(words[1].drop_meta(), Node::Symbol(_))
						&& crate::analyzer::type_word_kind(&words[0].drop_meta().name()).is_some() =>
				{
					Node::Key(Box::new(words[1].clone()), Op::Colon, Box::new(words[0].clone()))
				}
				_ => parameter,
			})
			.collect()
	}

	/// `= Some(T) | None`, `: red | rgb(int, int, int)`, Elm's `= Failure HttpError | …` on lines of their own: the
	/// offset after the `=` or `:` when the declaration lists alternatives one of which carries a payload
	pub(crate) fn variants_ahead(&self) -> Option<usize> {
		let blanks = (0..).take_while(|&offset| self.peek_char(offset).is_whitespace()).count();
		let (sign, next) = (self.peek_char(blanks), self.peek_char(blanks + 1));
		let opens = (sign == '=' && !matches!(next, '=' | '>')) || (sign == ':' && next != '=');
		if !opens {
			return None;
		}
		let start = blanks + 1;
		let (mut depth, mut alternatives, mut payload, mut words) = (0, false, false, 0);
		let mut offset = start;
		loop {
			let ch = self.peek_char(offset);
			let after_word = is_identifier_char(self.peek_char(offset - 1));
			match ch {
				'\0' | ';' if depth == 0 => break,
				'\n' if depth == 0 => {
					let blanks = (offset + 1..).take_while(|&at| self.peek_char(at).is_whitespace()).count();
					if self.peek_char(offset + 1 + blanks) != '|' {
						break;
					}
				}
				'(' if depth == 0 && after_word => {
					payload = true;
					depth += 1;
				}
				'(' | '[' | '{' => depth += 1,
				')' | ']' | '}' if depth == 0 => break,
				')' | ']' | '}' => depth -= 1,
				'|' if depth == 0 && self.peek_char(offset + 1) == '|' => offset += 1,
				'|' if depth == 0 => (alternatives, words) = (true, 0),
				_ if depth == 0 && is_identifier_char(ch) && !after_word => {
					words += 1;
					payload |= words > 1;
				}
				_ => {}
			}
			offset += 1;
		}
		(alternatives && payload).then_some(start)
	}

	/// The sum type `Name = A(T) | B | C(x: int)` (card sum-types): the class Name without fields and a class extending
	/// it for each variant with a payload, its fields in order (an unnamed one is `value`, several `value1`, `value2`…);
	/// a variant without payload is its name, a symbol (`None` stays ø)
	pub(crate) fn parse_sum_type(&mut self, type_name: String, type_parameters: &[String], start: usize) -> Node {
		self.advance_by(start);
		let mut declarations = vec![];
		let mut bare_variants = vec![];
		loop {
			self.skip_spaces();
			let variant = match self.parse_symbol() {
				Ok(name) => name,
				Err(message) => return error(&format!("a variant of {type_name} needs a name: {message}")),
			};
			let mut payload = if self.current_char() == '(' { statements(self.parse_bracketed('(')) } else { vec![] };
			// Elm's `Failure HttpError`: the payload's types after the name
			self.skip_spaces();
			while self.is_identifier_start(0) {
				payload.extend(self.parse_symbol().ok().map(Symbol));
				self.skip_spaces();
			}
			match payload.is_empty() {
				true => bare_variants.push(Symbol(variant)),
				false => declarations.push(self.variant_class(&type_name, variant, payload, type_parameters)),
			}
			let blanks = (0..).take_while(|&offset| self.peek_char(offset).is_whitespace()).count();
			if self.peek_char(blanks) != '|' {
				return crate::lowering::sum_variants::sum_type(type_name, bare_variants, declarations);
			}
			self.advance_by(blanks + 1);
		}
	}

	/// ` Shape { Circle(r), … }` after `enum`: a case followed by its values in parentheses
	pub(crate) fn enum_with_values_ahead(&self) -> bool {
		let mut offset = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		let name_length = (offset..).take_while(|&at| is_identifier_char(self.peek_char(at))).count();
		if name_length == 0 {
			return false;
		}
		offset += name_length;
		offset += (offset..).take_while(|&at| self.peek_char(at).is_whitespace()).count();
		if self.peek_char(offset) != '{' {
			return false;
		}
		let mut depth = 0;
		loop {
			match self.peek_char(offset) {
				'\0' => return false,
				'(' if depth == 1 && is_identifier_char(self.peek_char(offset - 1)) => return true,
				'(' | '[' | '{' => depth += 1,
				')' | ']' | '}' if depth == 1 => return false,
				')' | ']' | '}' => depth -= 1,
				_ => {}
			}
			offset += 1;
		}
	}

	/// `enum Shape { Circle(r), Rect(w, h), Dot }`, Swift's `case circle(radius: Double)` lines: each case with values a
	/// class extending Shape, the others its variants by name, as the sum type `Circle(r) | Rect(w, h) | Dot`
	pub(crate) fn parse_enum_with_values(&mut self) -> Node {
		self.skip_spaces();
		let type_name = or_return_error!(self.parse_symbol());
		self.skip_whitespace();
		let Node::List(cases, _, _) = self.parse_bracketed('{').drop_meta().clone() else {
			return error(&format!("enum {type_name} needs its cases in braces"));
		};
		let mut bare_variants = vec![];
		let mut declarations = vec![];
		for case in cases.iter().map(without_case_keyword) {
			match case.drop_meta() {
				Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => {
					declarations.push(self.variant_class(
						&type_name,
						items[0].drop_meta().name(),
						items[1..].to_vec(),
						&[],
					));
				}
				Node::Symbol(_) => bare_variants.push(case.drop_meta().clone()),
				other => {
					return error(&format!(
						"a case of enum {type_name} is a name with optional values, got {}",
						other.serialize()
					))
				}
			}
		}
		crate::lowering::sum_variants::sum_type(type_name, bare_variants, declarations)
	}

	pub(crate) fn variant_class(
		&mut self,
		type_name: &str,
		variant: String,
		payload: Vec<Node>,
		type_parameters: &[String],
	) -> Node {
		let count = payload.len();
		let field_name = |index: usize| {
			if count == 1 {
				VARIANT_FIELD.to_string()
			} else {
				format!("{VARIANT_FIELD}{}", index + 1)
			}
		};
		// `(T)` and `(radius: int)` arrive as groups of one
		let sole = |field: Node| match field.drop_meta() {
			Node::List(items, _, _) if items.len() == 1 => items[0].clone(),
			_ => field,
		};
		// P179: `rgb(r, g, b)` names its fields; `Some(T)`, `rgb(int, int, int)` and `Pair(Point, Point)` give types
		let is_type = |name: &str| {
			crate::analyzer::builtin_type_kind(name).is_some()
				|| type_parameters.iter().any(|parameter| parameter == name)
				|| self.declared_types.contains(name)
				|| name.starts_with(char::is_uppercase)
		};
		let mut untyped = type_parameters.to_vec();
		let fields: Vec<Node> = payload
			.into_iter()
			.map(sole)
			.enumerate()
			.map(|(index, field)| match field.drop_meta() {
				Node::Key(_, Op::Colon, _) => field,
				Node::Symbol(name) if !is_type(name) => {
					untyped.push(name.clone());
					Node::Key(Box::new(Symbol(name.clone())), Op::Colon, Box::new(field))
				}
				_ => Node::Key(Box::new(Symbol(field_name(index))), Op::Colon, Box::new(field)),
			})
			.collect();
		let body = Self::transform_fields_to_types(Node::List(fields, Bracket::Curly, Separator::Semicolon));
		self.declared_types.insert(variant.clone());
		let name = Symbol(variant).with_attribute(EXTENDS_KEYWORD, Symbol(type_name.to_string()));
		Node::Type { name: Box::new(name), body: Box::new(any_for_type_parameters(body, &untyped)) }
	}

	/// `<T>`, `<A, B>`, `[T]`: the names of the type parameters of a declared type
	pub(crate) fn parse_type_parameters(&mut self) -> Result<Vec<String>, String> {
		let close = if self.current_char() == '[' { ']' } else { '>' };
		self.advance(); // < or [
		let mut names = vec![];
		loop {
			self.skip_whitespace();
			names.push(self.parse_symbol()?);
			self.skip_whitespace();
			match self.current_char() {
				',' => self.advance(),
				ch if ch == close => {
					self.advance();
					return Ok(names);
				}
				other => return Err(format!("type parameters <{}…> end with {close}, got {other}", names.join(", "))),
			}
		}
	}

	/// A class body's members with their field types: `{x: int; double() := …}`. A body of one C-style method
	/// (`{int bar(){ 42 }}`) groups as the words `int` and `bar(){…}`: it is one member, not two fields
	pub(crate) fn class_body(block: Node) -> Node {
		let block = match block {
			Node::List(items, Bracket::Curly, Separator::Space) if is_typed_method(&items) => Node::List(
				vec![Node::List(items, Bracket::None, Separator::Space)],
				Bracket::Curly,
				Separator::Semicolon,
			),
			other => other,
		};
		Self::transform_fields_to_types(block)
	}

	/// Transform field definitions: Key(name, op, Symbol) -> Key(name, op, Type)
	/// Used for class/struct definitions to convert type names to Type nodes
	pub(crate) fn transform_fields_to_types(node: Node) -> Node {
		match node {
			// a keyword method (`def scaled(k) { side * k }`) or a constructor (`init(n) {…}`) in a class body: its body is
			// code, no field types
			Node::List(items, bracket, sep) if items.first().is_some_and(starts_code) => {
				Node::List(items, bracket, sep)
			}
			Node::List(items, bracket, sep) => {
				let mut transformed: Vec<Node> = with_type_phrases(without_ref_words(items))
					.into_iter()
					.map(Self::transform_fields_to_types)
					.collect();
				// the line `left: ref Node?` is the one field once its `ref` is read
				if bracket == Bracket::None
					&& transformed.len() == 1
					&& matches!(transformed[0].drop_meta(), Node::Key(_, Op::Colon, _))
				{
					return transformed.remove(0);
				}
				// the line `left: ref Node? right: ref Node?` among others: its fields are fields of the body
				// a typed field, not a property with its block (`age:{2026 - birthday} set{…}`)
				let is_field = |item: &Node| matches!(item.drop_meta(), Node::Key(_, Op::Colon, value) if !matches!(value.drop_meta(), Node::List(_, Bracket::Curly, _)));
				let row_of_fields = |item: &Node| matches!(item.drop_meta(), Node::List(row, Bracket::None, Separator::Space) if row.iter().all(is_field));
				if sep != Separator::Space && transformed.iter().any(row_of_fields) {
					transformed = transformed
						.into_iter()
						.flat_map(|item| match item.drop_meta() {
							Node::List(row, Bracket::None, Separator::Space) if row.iter().all(is_field) => row.clone(),
							_ => vec![item],
						})
						.collect();
				}
				Node::List(transformed, bracket, sep)
			}
			// a method in a class body (`greet() := …`) or a field's default (`tag: text = mark`): code, no field type
			Node::Key(name, op @ (Op::Define | Op::Assign), value) => Node::Key(name, op, value),
			Node::Key(name, op, value) => {
				let type_node = Self::symbol_to_type(*value);
				Node::Key(name, op, Box::new(type_node))
			}
			meta @ Node::Meta { .. } => meta.under_meta(Self::transform_fields_to_types),
			other => other,
		}
	}

	/// Convert a Symbol to a Type node (for type references)
	pub(crate) fn symbol_to_type(node: Node) -> Node {
		match node {
			Node::Symbol(s) => Node::Type { name: Box::new(Node::Symbol(s)), body: Box::new(Empty) },
			meta @ Node::Meta { .. } => meta.under_meta(Self::symbol_to_type),
			other => other,
		}
	}
}

/// A class body with each type parameter used as a type read as any type: a field `item:T` holds any value, a
/// parameter `with(x:T)` takes any
pub(crate) fn any_for_type_parameters(node: Node, parameters: &[String]) -> Node {
	let is_parameter = |node: &Node| match node.drop_meta() {
		Node::Symbol(name) => parameters.contains(name),
		Node::Type { name, body } => body.is_empty_node() && parameters.contains(&name.drop_meta().name()),
		_ => false,
	};
	match node {
		Node::Key(name, Op::Colon, kind) if is_parameter(&kind) => {
			Node::Key(name, Op::Colon, Box::new(Symbol(ANY_TYPE.to_string())))
		}
		Node::Key(head, Op::Define, body) => {
			Node::Key(Box::new(untyped_parameters(*head, &is_parameter)), Op::Define, body)
		}
		other => other.map_children(|child| any_for_type_parameters(child, parameters)),
	}
}

/// A method head with each parameter of a type parameter untyped: `with(x:T)` is `with(x)`
pub(crate) fn untyped_parameters(head: Node, is_parameter: &dyn Fn(&Node) -> bool) -> Node {
	match head {
		Node::Key(name, Op::Colon, kind) if is_parameter(&kind) => *name,
		other => other.map_children(|child| untyped_parameters(child, is_parameter)),
	}
}

/// Swift's `case circle(radius: Double)`: the case without its keyword
pub(crate) fn without_case_keyword(case: &Node) -> Node {
	match case.drop_meta() {
		Node::List(words, _, _) if words.len() == 2 && words[0].drop_meta().name() == CASE_KEYWORD => words[1].clone(),
		_ => case.clone(),
	}
}

/// The items of a block or parameter list `{a; b}`, `(a, b)`; a block of one statement `{fun sum() = x}` is that one
pub(crate) fn statements(block: Node) -> Vec<Node> {
	match block {
		Node::List(items, _, Separator::Comma | Separator::Semicolon | Separator::Newline) => items,
		Empty => vec![],
		Node::List(items, Bracket::Curly | Bracket::Round, separator) => {
			vec![Node::List(items, Bracket::None, separator)]
		}
		single => vec![single],
	}
}

/// `T, F: Fn(i32) -> i32` → `T F`: the names before each bound
pub(crate) fn type_parameter_names(written: &str) -> Vec<String> {
	let mut depth = 0;
	let mut parts = vec![String::new()];
	for ch in written.chars() {
		match ch {
			'<' | '(' => depth += 1,
			'>' | ')' => depth -= 1,
			',' if depth == 0 => parts.push(String::new()),
			_ => {}
		}
		if ch != ',' || depth != 0 {
			parts.last_mut().expect("one part").push(ch);
		}
	}
	parts
		.iter()
		.filter_map(|part| part.split(':').next().map(str::trim).filter(|name| !name.is_empty()).map(str::to_string))
		.collect()
}

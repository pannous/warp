//! Lambdas: MATLAB `@(x)`, backslash `\x ->`, stabby `->(x)`, lambdas held in fields
use super::*;

/// `{f: x => x * 2}`: `:` binds tighter than `=>`, so the entry parses as the lambda `(f:x) => x*2` with a parameter f of
/// type x; in an object a lowercase word that is no type after the key is the lambda's parameter: the field f holds
/// `x => x * 2`. A typed lambda `{x: int => x * 2}` or `{p: Person => p.name}` stays one.
pub(crate) fn field_holding_lambda(item: Node) -> Node {
	match item {
		meta @ Node::Meta { .. } => meta.under_meta(field_holding_lambda),
		Node::Key(head, Op::FatArrow, body) => match head.drop_meta() {
			Node::Key(key, Op::Colon, parameter)
				if matches!(key.drop_meta(), Symbol(_)) && is_parameter_word(parameter) =>
			{
				let lambda = Node::Key(parameter.clone(), Op::FatArrow, body);
				Node::Key(key.clone(), Op::Colon, Box::new(lambda))
			}
			_ => Node::Key(head, Op::FatArrow, body),
		},
		other => other,
	}
}

impl WarpParser {
	/// MATLAB's anonymous function `@(x) x.^2`: the lambda `x => x^2`
	pub(crate) fn parse_matlab_lambda(&mut self) -> Node {
		self.advance(); // '@'
		let parameters = self.parse_bracketed('(');
		self.skip_spaces();
		let body = self.parse_expr(Op::Assign.binding_power().1);
		Node::Key(Box::new(parameters), Op::FatArrow, Box::new(body))
	}

	/// `\x ->`, `\a b ->`: names and an arrow after the backslash (`\alpha` alone is an entity)
	pub(crate) fn backslash_lambda_ahead(&self) -> bool {
		let rest: String = self.chars[self.pos + 1..].iter().take_while(|ch| **ch != '\n').collect();
		let Some((names, _)) = rest.split_once("->") else {
			return false;
		};
		let names: Vec<&str> = names.split_whitespace().collect();
		!names.is_empty()
			&& names.iter().all(|name| {
				name.starts_with(char::is_alphabetic) && name.chars().all(|ch| ch.is_alphanumeric() || ch == '_')
			})
	}

	/// Haskell's `\x -> x * 2`, `\a b -> a + b`: the lambda
	pub(crate) fn parse_backslash_lambda(&mut self) -> Node {
		self.advance(); // '\\'
		let mut names = Vec::new();
		loop {
			self.skip_spaces();
			if self.current_char() == '-' && self.peek_char(1) == '>' {
				self.advance_by(2);
				break;
			}
			match self.at_identifier_start().then(|| self.parse_symbol().ok()).flatten() {
				Some(name) => names.push(Symbol(name)),
				None => return error("a lambda `\\x -> …` needs its arrow"),
			}
		}
		self.skip_spaces();
		let parameters = match names.len() {
			1 => names.remove(0),
			_ => Node::List(names, Bracket::Round, Separator::Colon),
		};
		let body = self.parse_expr(Op::Assign.binding_power().1);
		Node::Key(Box::new(parameters), Op::FatArrow, Box::new(body))
	}

	/// `<T, F: Fn(i32) -> i32>` glued to a function's name and followed by its parameters: its length
	pub(crate) fn generic_parameters_length(&self) -> Option<usize> {
		if self.current_char() != '<' {
			return None;
		}
		let mut depth = 0;
		for (offset, &ch) in self.chars[self.pos..].iter().enumerate() {
			match ch {
				'<' => depth += 1,
				'>' if offset > 0 && self.chars[self.pos + offset - 1] == '-' => {} // the arrow of `Fn(i32) -> i32`
				'>' => {
					depth -= 1;
					if depth == 0 {
						return (self.peek_char(offset + 1) == '(').then_some(offset + 1);
					}
				}
				'\n' | ';' | '{' => return None,
				_ => {}
			}
		}
		None
	}

	/// Ruby's stabby lambda `->(x) { x * x }`, `-> { 42 }`: the lambda `x => {x * x}`
	pub(crate) fn parse_stabby_lambda(&mut self) -> Node {
		self.advance_by(2);
		self.skip_spaces();
		let parameters = match self.current_char() {
			'(' => self.parse_bracketed('('),
			_ => Empty,
		};
		self.skip_spaces();
		if self.current_char() != '{' {
			return error("a stabby lambda `->(x) { … }` needs its body in braces");
		}
		let body = self.parse_bracketed('{');
		Node::Key(Box::new(parameters), Op::FatArrow, Box::new(body))
	}

	/// Python's `lambda a, b=2: a + b`, `lambda *xs: …`, `lambda: 42` at the cursor: the parameters as one group (the
	/// commas would split them into list items), then the body up to the end of the expression
	pub(in crate::warp_parser) fn parse_python_lambda(&mut self) -> Option<Node> {
		if !self.matches_keyword(PYTHON_LAMBDA_KEYWORD) || self.options.data_mode {
			return None;
		}
		let start = self.pos + PYTHON_LAMBDA_KEYWORD.len();
		let rest = &self.chars[start..];
		let colon = rest.iter().position(|ch| matches!(ch, ':' | '\n' | ';' | '{' | '[' | ')'))?;
		let parameters: String = rest[..colon].iter().collect();
		let names_a_variable = parameters.trim_start().starts_with('=');
		if rest[colon] != ':' || names_a_variable || rest.get(colon + 1) == Some(&'=') {
			return None;
		}
		let parameters = match parameters.trim() {
			"" => Empty,
			written => match parse(&format!("({written})")) {
				Node::Error(_) => return None,
				group => group,
			},
		};
		self.advance_by(start + colon + 1 - self.pos);
		self.skip_spaces();
		let body = self.parse_expr(Op::Assign.binding_power().1);
		let body = self.continue_expr(body, 0);
		Some(Node::Key(Box::new(parameters), Op::FatArrow, Box::new(body)))
	}
}

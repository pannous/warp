//! The hints of each axis: a form other than the canonical one of `style()` gets its hint
use super::*;

/// The type word in its canonical spelling: `str` and `String` become `string` when that is preferred
fn canonical_type_word(used: &str) -> &str {
	if style().prefer_string_over_str && STRING_TYPE_NAMES.contains(&used) {
		"string"
	} else {
		used
	}
}

/// Returns whether a hint was emitted, so that the type word is not hinted a second time
pub fn type_constructor(type_name: &str, value: &str) -> bool {
	let is_non_canonical = style().cast.instead_of(CastStyle::Constructor).is_some();
	if is_non_canonical {
		let original = format!("{}({})", type_name, value);
		let canonical = format!("{} as {}", value, canonical_type_word(type_name));
		hint(&original, &canonical, "postfix 'as' reads naturally: value as type");
	}
	is_non_canonical
}

/// 'as' operator when constructor style is preferred; returns whether a hint was emitted
pub fn as_operator(value: &str, type_name: &str) -> bool {
	let is_non_canonical = style().cast.instead_of(CastStyle::AsOperator).is_some();
	if is_non_canonical {
		let original = format!("{} as {}", value, type_name);
		let canonical = format!("{}({})", canonical_type_word(type_name), value);
		hint(&original, &canonical, "constructor style preferred for casts");
	}
	is_non_canonical
}

/// String type name variations (str, String -> string)
pub fn string_type(used: &str) {
	let s = style();
	if s.prefer_string_over_str && STRING_TYPE_NAMES.contains(&used) {
		hint(used, "string", "use lowercase 'string' for the string type");
	}
}

/// The quote character a string literal was written with
pub fn quotes(used: char, content: &str) {
	let used_style = match used {
		'\'' => QuoteStyle::Single,
		'"' => QuoteStyle::Double,
		_ => return,
	};
	let reason = match style().quotes.instead_of(used_style) {
		Some(QuoteStyle::Single) => "single quotes preferred for strings",
		Some(QuoteStyle::Double) => "double quotes preferred for strings",
		_ => return,
	};
	let canonical_quote = text_quote();
	hint(&format!("{used}{content}{used}"), &format!("{canonical_quote}{content}{canonical_quote}"), reason);
}

/// The operator as written in the source; `is_prefix` tells `!x` from an infix use
pub fn operator(written: &str, is_prefix: bool) {
	let mut characters = written.chars();
	if let (Some(glyph), None) = (characters.next(), characters.next()) {
		if let Some(op) = glyph_operator(glyph) {
			match op.as_str() {
				"and" => logical_operator(written, "and", AND_SYMBOLS),
				"or" => logical_operator(written, "or", OR_SYMBOLS),
				"xor" => {}
				canonical if canonical != written => hint(written, canonical, "standard spelling of the operator"),
				_ => {}
			}
			return;
		}
	}
	match (written, is_prefix) {
		("¬&", false) => hint(written, "nand", "standard spelling of the operator"),
		("&&" | "&" | "and", false) => logical_operator(written, "and", AND_SYMBOLS),
		("||" | "|" | "or", false) => logical_operator(written, "or", OR_SYMBOLS),
		("!" | "not", true) => not_operator(written),
		("**" | "^", false) => power_operator(written),
		("?", false) => conditional(true),
		("if", true) => conditional(false),
		_ => {}
	}
}

/// `and` / `&&`, `or` / `||`: the hint toward the style's spelling; `symbols` = the canonical symbol, its short
/// form, then the logic glyphs
fn logical_operator(used: &str, word: &str, symbols: [&str; 4]) {
	match style().logical {
		LogicalStyle::Words if symbols.contains(&used) => hint(used, word, "word operators are more readable"),
		LogicalStyle::Symbols if used == word || symbols[2..].contains(&used) => {
			hint(used, symbols[0], "symbol operators preferred")
		}
		_ => {}
	}
}

/// ! or not operator
pub fn not_operator(used: &str) {
	let s = style();
	match (used, s.logical) {
		("!", LogicalStyle::Words) => hint("!", "not", "word operators are more readable"),
		("not", LogicalStyle::Symbols) => hint("not", "!", "symbol operators preferred"),
		_ => {}
	}
}

/// Power operator ** vs ^
pub fn power_operator(used: &str) {
	let s = style();
	match (used, s.power) {
		("**", PowerStyle::Caret) => hint("**", "^", "use ^ for exponentiation"),
		("^", PowerStyle::DoubleStar) => hint("^", "**", "use ** for exponentiation"),
		_ => {}
	}
}

/// Ternary vs if-then-else
pub fn conditional(used_ternary: bool) {
	let s = style();
	match (used_ternary, s.conditional) {
		// shown until the user acknowledges it once (card hint-dismiss)
		(true, ConditionalStyle::IfThenElse) => crate::errors::diagnostic::educate_once(
			CONDITIONAL_TOPIC,
			"x ? y : z",
			"if x then y else z",
			"if-then-else is more readable",
		),
		(false, ConditionalStyle::Ternary) => {
			hint("if x then y else z", "x ? y : z", "ternary operator is more concise")
		}
		_ => {}
	}
}

/// Variable definition keywords
pub fn var_keyword(used: &str, name: &str, value: &str) {
	let used_style = match used {
		"let" => VarStyle::Let,
		"var" => VarStyle::Var,
		":=" => VarStyle::ColonEquals,
		_ => return,
	};
	let Some(preferred) = style().var_def.instead_of(used_style) else {
		return;
	};
	let spell = |var_style: VarStyle| match var_style {
		VarStyle::ColonEquals => format!("{name} := {value}"),
		VarStyle::Let => format!("let {name} = {value}"),
		VarStyle::Var => format!("var {name} = {value}"),
		VarStyle::Any => format!("{name} = {value}"),
	};
	let reason = match preferred {
		// a style hint never changes the meaning: let and := cannot change, a variable stays one (card var-hint)
		VarStyle::ColonEquals | VarStyle::Let if used_style == VarStyle::Var => {
			return hint(
				&spell(used_style),
				&spell(VarStyle::Any),
				"a variable is a plain assignment (let and := cannot change)",
			);
		}
		VarStyle::ColonEquals => "use := for definition",
		VarStyle::Let => "use 'let' for definition",
		VarStyle::Var => "use 'var' for definition",
		VarStyle::Any => return,
	};
	match used_style {
		// shown until the user acknowledges it once (diagnostic::educate_once)
		VarStyle::Let => crate::errors::diagnostic::educate_once(
			LET_TOPIC,
			&spell(used_style),
			&spell(preferred),
			&format!("{reason}; in warp `let` is immutable (unlike JS), use var or plain = for variables that change"),
		),
		_ => hint(&spell(used_style), &spell(preferred), reason),
	}
}

/// The canonical spelling of the list of `element_words` (`["int"]`, or `["list", "int"]` for a list of lists) in `list_style`;
/// `None` when the style cannot spell it (`ints` has no plural for a list of lists)
fn list_type_spelling(list_style: ListTypeStyle, element_words: &[&str]) -> Option<String> {
	match (list_style, element_words) {
		(ListTypeStyle::Plural, [element]) => crate::analyzer::type_word_kind(element).map(|_| format!("{element}s")),
		(ListTypeStyle::Plural | ListTypeStyle::Any, _) => None,
		(ListTypeStyle::Generic, words) => {
			let (last, nested) = words.split_last()?;
			let arguments = nested.iter().rev().fold(last.to_string(), |inner, word| format!("{word}<{inner}>"));
			Some(format!("{LIST_WORD}<{arguments}>"))
		}
		(ListTypeStyle::Of, words) => Some(format!("{LIST_WORD} {OF_WORD} {}", words.join(" of "))),
		(ListTypeStyle::Bracket, words) => {
			let (last, nested) = words.split_last()?;
			Some(nested.iter().rev().fold(format!("[{last}]"), |inner, _| format!("[{inner}]")))
		}
	}
}

/// A list type written in `used` style: `list<int>` and `list of int` when `ints` is preferred, `ints` when the words are
pub fn list_type(used: ListTypeStyle, element_words: &[&str]) {
	let Some(preferred) = style().list_type.instead_of(used) else {
		return;
	};
	if let (Some(original), Some(canonical)) =
		(list_type_spelling(used, element_words), list_type_spelling(preferred, element_words))
	{
		let reason = match preferred {
			ListTypeStyle::Plural => "a plural type word is a list of that type",
			ListTypeStyle::Generic => "angle brackets apply the list type to its element type",
			ListTypeStyle::Of => "'list of' reads as a phrase",
			ListTypeStyle::Bracket => "square brackets around the element type read as a list",
			ListTypeStyle::Any => return,
		};
		hint(&original, &canonical, reason);
	}
}

/// Function definition keywords
pub fn function_keyword(used: &str, name: &str, params: &str, written_body: BodyForm) {
	let used_style = match used {
		"def" => FunctionStyle::Def,
		"define" => FunctionStyle::Define,
		"fn" => FunctionStyle::Fn,
		"fun" => FunctionStyle::Fun,
		"function" => FunctionStyle::Function,
		":=" => FunctionStyle::ColonEquals,
		_ => return,
	};
	let Some(preferred) = style().function_def.instead_of(used_style) else {
		return;
	};
	let spell = |function_style: FunctionStyle, body: BodyForm| {
		let body_text = match body {
			BodyForm::Colon => ": ...",
			BodyForm::Assign => " = ...",
			BodyForm::Block => " { ... }",
		};
		match function_style {
			FunctionStyle::ColonEquals | FunctionStyle::Any => format!("{name}({params}) := ..."),
			FunctionStyle::Def => format!("def {name}({params}){body_text}"),
			FunctionStyle::Define => format!("define {name}({params}){body_text}"),
			FunctionStyle::Fn => format!("fn {name}({params}){body_text}"),
			FunctionStyle::Fun => format!("fun {name}({params}){body_text}"),
			FunctionStyle::Function => format!("function {name}({params}){body_text}"),
		}
	};
	let conventional_body = |function_style: FunctionStyle| match function_style {
		FunctionStyle::Def | FunctionStyle::Define => BodyForm::Colon,
		FunctionStyle::Fn | FunctionStyle::Fun | FunctionStyle::ColonEquals | FunctionStyle::Any => BodyForm::Assign,
		FunctionStyle::Function => BodyForm::Block,
	};
	let reason = match preferred {
		FunctionStyle::ColonEquals => "short := form preferred",
		FunctionStyle::Def => "'def' keyword preferred",
		FunctionStyle::Define => "'define' keyword preferred",
		FunctionStyle::Fn => "'fn' keyword preferred",
		FunctionStyle::Fun => "'fun' keyword preferred",
		FunctionStyle::Function => "'function' keyword preferred",
		FunctionStyle::Any => return,
	};
	hint(&spell(used_style, written_body), &spell(preferred, conventional_body(preferred)), reason);
}

/// `idx` shifted by `offset` for the other index base: `s[0]` is `s#1`, brackets count from 0 and `#` from 1
fn shifted_index(idx: &str, offset: i64) -> String {
	match idx.parse::<i64>() {
		Ok(number) => (number + offset).to_string(),
		Err(_) => format!("({idx}{offset:+})"),
	}
}

/// Bracket indexing (counts from 0) vs hash indexing (counts from 1)
pub fn index_operator(var: &str, idx: &str, used_bracket: bool) {
	let looks_up_a_key = idx.starts_with(['"', '\'']); // `ages["alice"]` counts no position
													// cards g-2WPo, g_YiSA (user): only a number, where `xs#2` is as short as `xs[1]`; `xs[i]` and `xs[i+1]` stay
	if looks_up_a_key || !is_number_index(idx) {
		return;
	}
	let s = style();
	match (used_bracket, s.index) {
		(true, IndexStyle::Hash) => {
			let original = format!("{}[{}]", var, idx);
			let canonical = format!("{}#{}", var, shifted_index(idx, 1));
			hint(&original, &canonical, "use # for indexing");
		}
		(false, IndexStyle::Bracket) => {
			let original = format!("{}#{}", var, idx);
			let canonical = format!("{}[{}]", var, shifted_index(idx, -1));
			hint(&original, &canonical, "use [] for indexing");
		}
		_ => {}
	}
}

fn is_number_index(idx: &str) -> bool {
	let digits = idx.strip_prefix('-').unwrap_or(idx);
	!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
}

/// Length method vs # operator
pub fn length_operator(var: &str, used_method: bool) {
	let s = style();
	match (used_method, s.index) {
		(true, IndexStyle::Hash) => {
			let original = format!("{}.length()", var);
			let canonical = format!("#{}", var);
			hint(&original, &canonical, "use # prefix for length");
		}
		(false, IndexStyle::Bracket) => {
			let original = format!("#{}", var);
			let canonical = format!("{}.length()", var);
			hint(&original, &canonical, "use .length() for length");
		}
		_ => {}
	}
}

/// `xs += [3]` of an item evidently no list appends it as `xs += 3` does (decision list-append-plus); an item of unknown
/// kind keeps its brackets, as a list item would be concatenated without them
pub fn appended_item(target: &str, item: &Node) {
	let item_text = match item.drop_meta() {
		Node::Char(character) => format!("{0}{character}{0}", text_quote()),
		Node::Number(_) | Node::Text(_) | Node::True | Node::False => operand_text(item),
		_ => return,
	};
	hint(
		&format!("{target} += [{item_text}]"),
		&format!("{target} += {item_text}"),
		"an item is appended without brackets",
	);
}

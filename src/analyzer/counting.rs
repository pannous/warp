//! Counting words and phrases: count/length/size of a value, units, ranges

use super::*;
use crate::words::PRINT_WORD;

/// Statement words whose argument is never their property: `return count` is no `return.count`
pub(crate) const PROPERTYLESS_KEYWORDS: [&str; 6] = ["return", "yield", PRINT_WORD, "println", "puts", "not"];
/// The words counting a value's elements, as `count x`, `x.size`, `len(x)`: the one table every pass asks, through
/// is_counting_word (a class's own size method is looked up in this order)
pub(crate) const COUNTING_WORDS: [&str; 4] = ["size", "count", "len", "length"];
/// `byte_size(x)`, `x.byte_size`: the bytes of x, as `x.bytes` (size is the element count, user decision P40)
pub(crate) const BYTE_SIZE: &str = "byte_size";
/// `x.number` counts too, but `number x` is the type conversion, not a count
pub(crate) const NUMBER_PROPERTY: &str = "number";

/// `count x`, `length x`, `size x`: the runtime function counting the elements of x; a text counts its graphemes
/// (user-perceived characters). Bytes are only counted by an explicit unit (`x.bytes`); a user function of that name wins.
pub fn counting_function(name: &str, ctx: &Context) -> Option<&'static str> {
	if ctx.user_functions.contains_key(name) {
		return None;
	}
	if name == BYTE_SIZE {
		return Some("node_bytes");
	}
	is_counting_word(name).then_some("node_count")
}

/// `count`, `length`, `size`, `len`: a word counting its argument (`number x` is a cast)
pub(crate) fn is_counting_word(name: &str) -> bool {
	COUNTING_WORDS.contains(&name)
}

/// `x.count`, `x.length`, `x.size`, and the explicit units `x.bytes` (memory), `x.codepoints`, `x.graphemes` (`x.chars` is the list of characters)
pub fn counting_method(name: &str, ctx: &Context) -> Option<&'static str> {
	match name {
		_ if is_counting_property(name) => Some("node_count"),
		"bytes" | BYTE_SIZE => Some("node_bytes"),
		"codepoints" => Some("text_codepoint_count"),
		"graphemes" => Some("text_grapheme_count"),
		_ => counting_function(name, ctx),
	}
}

/// The counting method of a text unit, named in the singular or the plural: `byte`, `chars`, `codepoint`, `graphemes`.
/// A char is a character (glyph), a grapheme cluster as `x.chars`; a code point is its own unit (`"שָלוֹם"` is 4
/// characters, 6 code points: decision chars-codepoints amended).
pub(crate) fn text_unit(word: &str) -> Option<&'static str> {
	match word {
		"byte" | "bytes" => Some("bytes"),
		"codepoint" | "codepoints" => Some("codepoints"),
		"char" | "chars" | "character" | "characters" | "glyph" | "glyphs" | "grapheme" | "graphemes" => {
			Some("graphemes")
		}
		_ => None,
	}
}

/// The items after the first `skip` as one node: the item itself, or the list of them
pub(super) fn rest_of(items: &[Node], skip: usize, bracket: &Bracket, separator: &Separator) -> Node {
	match &items[skip..] {
		[single] => single.clone(),
		rest => Node::List(rest.to_vec(), bracket.clone(), separator.clone()),
	}
}

/// A text seen in one unit, `byte in t` or `t as bytes`: counting it counts that unit, `t.bytes`
pub(crate) fn unit_count(node: &Node) -> Option<Node> {
	let count_of = |text: Node, unit: &Node| {
		let Node::Symbol(word) = unit.drop_meta() else {
			return None;
		};
		Some(Node::Key(Box::new(text), Op::Dot, Box::new(Node::Symbol(text_unit(word)?.to_string()))))
	};
	match node.drop_meta() {
		Node::List(items, _, _) if items.len() == 1 => unit_count(&items[0]),
		Node::List(items, bracket, separator)
			if items.len() >= 3 && items[1].is_symbol("in") && text_unit(&items[0].name()).is_some() =>
		{
			count_of(rest_of(items, 2, bracket, separator), &items[0])
		}
		Node::Key(text, Op::As, unit) => count_of(text.as_ref().clone(), unit),
		// `"äb" in bytes`: the unit last
		Node::List(items, bracket, separator) if items.len() >= 3 && items[items.len() - 2].is_symbol("in") => {
			let (unit, text) = (&items[items.len() - 1], &items[..items.len() - 2]);
			count_of(rest_of(text, 0, bracket, separator), unit)
		}
		_ => None,
	}
}

/// `#bytes in t`, parsed as the items `#bytes`, `in`, `t`: the count of that unit, `t.bytes`
pub(crate) fn hashed_unit_count(items: &[Node]) -> Option<Node> {
	let [hashed, keyword, _, ..] = items else { return None };
	let Node::Key(empty, Op::Hash, unit) = hashed.drop_meta() else {
		return None;
	};
	if !empty.is_empty_node() || !keyword.is_symbol("in") {
		return None;
	}
	let Node::Symbol(word) = unit.drop_meta() else {
		return None;
	};
	let counted = rest_of(items, 2, &Bracket::None, &Separator::Space);
	Some(Node::Key(Box::new(counted), Op::Dot, Box::new(Node::Symbol(text_unit(word)?.to_string()))))
}

/// `x size`, `x count`, `x length`, `x number`: a counting property word after a name is the getter `x.size`,
/// unless the name is a keyword (`return count`) or the word a variable of the program
pub(super) fn property_of_name(items: &[Node], separator: &Separator, variables: &HashSet<String>) -> Option<Node> {
	let [name, property] = items else { return None };
	let (Node::Symbol(name_text), Node::Symbol(property_text)) = (name.drop_meta(), property.drop_meta()) else {
		return None;
	};
	let names_no_object = is_counting_property(name_text) || PROPERTYLESS_KEYWORDS.contains(&name_text.as_str());
	if *separator != Separator::Space
		|| names_no_object
		|| !is_counting_property(property_text)
		|| variables.contains(property_text)
	{
		return None;
	}
	Some(Node::Key(Box::new(name.clone()), Op::Dot, Box::new(property.clone())))
}

/// `print chars in "hello"`: a name no variable has, in a collection, prints each item as `for chars in "hello": print it`
pub(crate) fn print_walk(items: &[Node], variables: &HashSet<String>) -> Option<Node> {
	let [print, phrase] = items else { return None };
	let Node::List(phrase, Bracket::None | Bracket::Round, _) = phrase.drop_meta() else {
		return None;
	};
	// `chars in "hello"` arrives as the words or as `chars (in "hello")`, optionally after `all`
	let phrase = match phrase.as_slice() {
		[all, rest @ ..] if all.is_symbol("all") => rest,
		words => words,
	};
	let (name, in_word, collection) = match phrase {
		[name, in_word, collection] => (name, in_word, collection),
		[name, rest] => match rest.drop_meta() {
			Node::List(rest, _, _) if rest.len() == 2 => (name, &rest[0], &rest[1]),
			_ => return None,
		},
		_ => return None,
	};
	let Node::Symbol(name) = name.drop_meta() else {
		return None;
	};
	if !print.is_symbol("print") || !in_word.is_symbol("in") || variables.contains(name) {
		return None;
	}
	let body = Node::List(vec![print.clone(), Node::Symbol(name.clone())], Bracket::Curly, Separator::Space);
	Some(Node::List(
		vec![Node::Symbol("for".into()), Node::Symbol(name.clone()), in_word.clone(), collection.clone(), body],
		Bracket::None,
		Separator::Space,
	))
}

/// A count in a unit, read after of_lookups made `a of b` into `b.a`: `count bytes of t` (`count t.bytes`), `number of bytes in t` (`bytes.number in t`) and `size of t in bytes`
/// (`t.size in bytes`) are all `t.bytes`
pub(crate) fn counting_phrase(
	items: &[Node],
	bracket: &Bracket,
	separator: &Separator,
	variables: &HashSet<String>,
) -> Option<Node> {
	if let Some(property) = property_of_name(items, separator, variables) {
		return Some(property);
	}
	let in_unit = |counted: Node, unit: &Node| {
		let unit = text_unit(unit.symbol_name()?)?;
		Some(Node::Key(Box::new(counted), Op::Dot, Box::new(Node::Symbol(unit.to_string()))))
	};
	match items {
		[count, counted] if count.symbol_name().is_some_and(is_counting_property) => {
			let (object, member) = dotted(counted)?;
			in_unit(object.clone(), &Node::Symbol(member.to_string()))
		}
		[counted, keyword, rest @ ..] if keyword.is_symbol("in") && !rest.is_empty() => {
			let (object, member) = dotted(counted)?;
			if !is_counting_property(member) {
				return None;
			}
			match rest {
				[unit] if text_unit(&unit.name()).is_some() => in_unit(object.clone(), unit),
				_ => in_unit(rest_of(items, 2, bracket, separator), object),
			}
		}
		_ => None,
	}
}

/// `object.member` with a word as member
fn dotted(node: &Node) -> Option<(&Node, &str)> {
	let Node::Key(object, Op::Dot, member) = node.drop_meta() else {
		return None;
	};
	Some((object, member.symbol_name()?))
}

/// The most numbers a range of literals is written out as at compile time; a longer one is collected at run time like
/// a range of computed bounds (a literal list of 100000 numbers overflowed the compiler's stack)
const LITERAL_RANGE_MAX_LENGTH: i64 = 1000;

/// The list a range of integer literals stands for: `1..4` is [1 2 3], `1…4` and `1 to 4` are [1 2 3 4]; not an empty,
/// computed or long range
pub(crate) fn range_elements(range: &Node) -> Option<Node> {
	let Node::Key(start, op @ (Op::Range | Op::To), end) = range.drop_meta() else {
		return None;
	};
	let (Node::Number(Number::Int(start)), Node::Number(Number::Int(end))) = (start.drop_meta(), end.drop_meta())
	else {
		return None;
	};
	let last = if *op == Op::To { *end } else { end - 1 };
	(start <= &last && last - start < LITERAL_RANGE_MAX_LENGTH)
		.then(|| Node::List((*start..=last).map(Node::int).collect(), Bracket::Square, Separator::Space))
}

/// The name a range of computed bounds collects its list under where no variable is assigned (`print a..b`)
pub(crate) const RANGE_VALUE: &str = "range_value";

/// `xs = a..b` of computed bounds: `(xs·range = ø; for xs·item in a..b { xs·range = xs·range + [xs·item] }; xs·range)`
pub(crate) fn computed_range(target: &Node, range: &Node) -> Option<Node> {
	let Node::Symbol(name) = target.drop_meta() else {
		return None;
	};
	if !matches!(range.drop_meta(), Node::Key(_, Op::Range | Op::To, _)) || range_elements(range).is_some() {
		return None;
	}
	let symbol = |suffix: &str| Node::Symbol(format!("{name}{TEMPORARY_SEPARATOR}{suffix}"));
	let (items, item) = (symbol("range"), symbol("item"));
	let appended = Node::Key(
		Box::new(items.clone()),
		Op::Add,
		Box::new(Node::List(vec![item.clone()], Bracket::Square, Separator::Space)),
	);
	let body = Node::List(
		vec![Node::Key(Box::new(items.clone()), Op::Assign, Box::new(appended))],
		Bracket::Curly,
		Separator::Semicolon,
	);
	let walk = Node::List(
		vec![Node::Symbol("for".into()), item, Node::Symbol("in".into()), range.clone(), body],
		Bracket::None,
		Separator::Space,
	);
	let start = Node::Key(Box::new(items.clone()), Op::Assign, Box::new(Node::Empty));
	Some(Node::List(vec![start, walk, items], Bracket::Round, Separator::Semicolon))
}

pub(super) fn require_counter(ctx: &mut Context, counter: &'static str) {
	if counter == "node_bytes" {
		ctx.required_functions.insert("node_count");
	}
	ctx.required_functions.insert(counter);
}

pub(crate) fn is_counting_property(word: &str) -> bool {
	word == NUMBER_PROPERTY || is_counting_word(word)
}

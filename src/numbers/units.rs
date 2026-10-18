//! Unit words as values (wiki/unit.md): `3km` is `3*km`, `1 m + 1km` is `1001 m`.
//! The units and their scales, the words a program writes them in, and the lowerings around them; static_units checks
//! the units of a program and runs its amounts as exact SI numbers. The unit shown is the finest one written:
//! `3km+10m` is `3010m`. A quantity is an amount times units to powers: `6 m²`, `6 m·kg`, `5 km/h` (km·h⁻¹), `1 kg·m/s²`.

use crate::compiler::meta::DataValue;
use crate::extensions::numbers::Number;
use crate::node::{error, Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashMap;
mod definitions;
pub use definitions::program_unit_lines;
mod quantity_arithmetic;
pub use quantity_arithmetic::*;

pub mod static_units;

use crate::extensions::reals::{superscript, Rational};
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use std::fmt;

/// A base quantity of lib/units.warp (`quantity length meter`, card unit-shallow): its place among them, in written
/// order; lib/units.warp declares them, any word whose anchor is a new unit (`quantity pixels px`)
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Dimension(u8);

/// The base quantities the compiler itself reads: durations (`sleep 2s`) and temperatures (their zero)
const DURATION_QUANTITY: &str = "duration";
const TEMPERATURE_QUANTITY: &str = "temperature";

impl Dimension {
	/// The base quantity lib/units.warp declares by `name`
	fn named(name: &str) -> Dimension {
		let known = definitions::definitions().dimensions.iter().find(|(known, _)| *known == name);
		known.map(|(_, dimension)| *dimension).unwrap_or_else(|| panic!("lib/units.warp declares no quantity {name}"))
	}

	fn duration() -> Dimension {
		Dimension::named(DURATION_QUANTITY)
	}

	fn temperature() -> Dimension {
		Dimension::named(TEMPERATURE_QUANTITY)
	}

	fn name(self) -> &'static str {
		let known = definitions::definitions().dimensions.iter().find(|(_, dimension)| *dimension == self);
		known.map_or("?", |(name, _)| name)
	}
}

/// `Length`: the quantity's name, capitalized (the Lean model's dimension names)
impl fmt::Debug for Dimension {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut name = self.name().chars();
		let first = name.next().map(|first| first.to_uppercase().collect::<String>()).unwrap_or_default();
		write!(formatter, "{first}{}", name.as_str())
	}
}

#[derive(Debug, PartialEq)]
/// `factor` counts the smallest step of the dimension, so every unit is a whole number of it (definitions.rs: the least
/// common multiple of the dimension's denominators; length in 0.2 mm, 1 ft = 1524, temperature in 1/180 K, 1 °F = 100).
/// `zero` is where the unit's 0 lies, in steps above the dimension's zero: 0 °C is 273.15 K, 49_167 steps
struct Unit {
	name: &'static str,
	dimension: Dimension,
	factor: i64,
	zero: i64,
}

/// The units of lib/units.warp (definitions.rs)
fn units() -> &'static [Unit] {
	&definitions::definitions().units
}

/// The sign before a temperature unit's letter: °C, °F
pub const DEGREE: char = '°';

/// Unit words standing for a unit expression (lib/units.warp): `60 mph` is `60 mi/h`, `440Hz` is `440/s` (lib/sound.warp),
/// an angle `30°` is the exact `30π/180` (card angles-degrees)
fn unit_aliases() -> &'static [(&'static str, &'static str)] {
	&definitions::definitions().aliases
}

/// The long names a conversion target may use, singular or plural: `2 h in minutes` (P36), lib/units.warp's `name hour :=
/// h` lines. Quantities keep the short names, `2 minutes` stays a duration of the time module.
fn long_names() -> &'static [(&'static str, &'static str)] {
	&definitions::definitions().long_names
}

// `100 cm in m`: the word of a conversion written with spaces (`as` is an operator)
/// `now` compared with a duration is its local time of day, unless the program names something so (card time-day-value);
/// a bare `time` is the type (user decision print-time), so `time < 12h` fails with the fix `now < 12h`
use crate::time::{NOW_WORD, TIME_WORD};
use crate::words::IN_WORD;

fn unit_named(name: &str) -> Option<&'static Unit> {
	let name = match name.strip_prefix(AFTER_NUMBER) {
		Some(unit) => unit,
		None if crate::compiler::compile_context::read(|context| context.shadowed_units.contains(name)) => return None,
		None => name,
	};
	units().iter().find(|unit| unit.name == name)
}

/// The mark of a unit word after a number whose name the program also defines (`s = sample(…); 0.5s`): decision
/// unit-definitions, unit symbols bind after a number, the program's names everywhere else (card variable-named-like)
const AFTER_NUMBER: char = '\u{2060}';

/// Each unit word after a number, or continuing a unit after one (`3 km/h`, `9.8 m/s²`), marked where the program
/// also defines its name, so the shadowing of that name spares it
fn with_units_after_numbers(program: Node) -> Node {
	let names = defined_unit_names(&program);
	if names.is_empty() {
		return program;
	}
	let units_used = crate::modules::says_use(&program, crate::modules::UNITS_MODULE);
	let mut callables = std::collections::HashSet::new();
	collect_callable_names(&program, &mut callables);
	let variables: std::collections::HashSet<String> = names.difference(&callables).cloned().collect();
	let shadowed = ShadowedUnits { names, variables, units_used };
	let program = marked_after_numbers(program, &shadowed);
	// `2000ms as s` where only a function binds s (its parameter): elsewhere it is the unit. A module alias
	// `use math as mm` stays the alias
	let local_names = crate::program_names::bound_in_functions(&program);
	crate::program_names::rewritten_by_scope(program, &|node, scope| match node {
		Node::Key(value, Op::As, unit) if is_unit_type(&unit) => {
			let is_free_local = |name: &&String| local_names.contains(*name) && !scope.takes(name);
			let names = shadowed.names.iter().filter(is_free_local).cloned().collect();
			let free = ShadowedUnits { names, variables: shadowed.variables.clone(), units_used };
			Node::Key(value, Op::As, Box::new(marked_unit(*unit, &free)))
		}
		other => other,
	})
}

/// The unit names the program also defines, those of them that are variables or parameters, and whether it says
/// `use units`, which makes such a variable's name after a number the unit (decision litre-suffix)
struct ShadowedUnits {
	names: std::collections::HashSet<String>,
	variables: std::collections::HashSet<String>,
	units_used: bool,
}

impl ShadowedUnits {
	fn contains(&self, name: &str) -> bool {
		self.names.contains(name)
	}
}

fn marked_after_numbers(node: Node, shadowed: &ShadowedUnits) -> Node {
	match node {
		Node::Key(left, op @ (Op::Mul | Op::Div), right)
			if (op == Op::Mul && is_amount(&left) && is_juxtaposed(&right)) || is_quantity(&left) =>
		{
			if op == Op::Mul {
				if let Some(refusal) = unit_beside_variable(&left, &right, shadowed) {
					return refusal;
				}
			}
			let left = marked_after_numbers(*left, shadowed);
			Node::Key(Box::new(left), op, Box::new(marked_unit(*right, shadowed)))
		}
		// `v: km/h`, a compound unit as a variable's declared type (`Err(m): m` is a match arm)
		Node::Key(target, Op::Colon, unit)
			if matches!(target.drop_meta(), Node::Symbol(_))
				&& matches!(unit.drop_meta(), Node::Key(_, Op::Mul | Op::Div, _))
				&& is_unit_type(&unit) =>
		{
			Node::Key(target, Op::Colon, Box::new(marked_unit(*unit, shadowed)))
		}
		other => other.map_children(|child| marked_after_numbers(child, shadowed)),
	}
}

/// `km/h`, `m`: unit words alone
fn is_unit_type(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(left, Op::Mul | Op::Div, right) => is_unit_type(left) && is_unit_term(right),
		other => {
			is_unit_term(other)
				&& !matches!(other, Node::Symbol(name) if crate::lowering::declaration_forms::is_declaration_type(&Node::Symbol(name.clone())))
		}
	}
}

/// `m = 5; 3m`: a unit glued to a number, named like a variable of the program. With `use units` it is 3 meters (user
/// decision unit-after-number) and the hint says how to multiply by the variable; without, an error with both fixes
/// (decision litre-suffix)
fn unit_beside_variable(amount: &Node, unit: &Node, shadowed: &ShadowedUnits) -> Option<Node> {
	let Node::Symbol(name) = unit.drop_meta() else {
		return None;
	};
	if !shadowed.variables.contains(name) {
		return None;
	}
	let amount = amount.drop_meta().serialize();
	let (written, multiplied) = (format!("{amount}{name}"), format!("{amount}*{name}"));
	if !shadowed.units_used {
		return Some(error(&format!(
			"{written}: {name} is both a unit and a variable here; add `use units` for the unit, or write {multiplied} to multiply"
		)));
	}
	crate::normalize::set_position_of(unit);
	let reason =
		format!("{written} is {amount} of the unit {name}; to multiply by the variable {name} write {multiplied}");
	crate::normalize::advise(&written, &multiplied, &reason, None);
	None
}

/// `3`, `2*1000`: an amount a unit word after it binds to
fn is_amount(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Number(_) => true,
		Node::Key(left, Op::Mul | Op::Div, right) => is_amount(left) && is_amount(right),
		_ => false,
	}
}

/// `3 km`, `3 km/h`: an amount times unit words
fn is_quantity(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(left, Op::Mul, right) if is_amount(left) => is_juxtaposed(right) && is_unit_term(right),
		Node::Key(left, Op::Mul | Op::Div, right) => is_quantity(left) && is_unit_term(right),
		_ => false,
	}
}

/// A unit word the parser joined to its number (`3m`, `3 m`), not the right side of a written `*` (`3 * m`)
fn is_juxtaposed(node: &Node) -> bool {
	let mut layer = node;
	while let Node::Meta { node: inner, .. } = layer {
		if let Some(position) = layer.get_lineinfo() {
			return position.after_number;
		}
		layer = inner;
	}
	false
}

/// `90 min`, `2 h`: a whole amount of a time unit the parser joined to it, as the time module's exact duration
/// (decision duration-units); none for any other quantity
pub fn duration_quantity(node: &Node) -> Option<crate::time::Duration> {
	let Node::Key(amount, Op::Mul, unit) = node.drop_meta() else {
		return None;
	};
	let Node::Number(Number::Int(count)) = amount.drop_meta() else {
		return None;
	};
	let name = unit.drop_meta().symbol_name()?;
	let unit_word = unit_named(name.strip_prefix(AFTER_NUMBER).unwrap_or(name))?;
	let duration = quantity_arithmetic::unit_duration(unit_word)?;
	is_juxtaposed(unit).then(|| duration.times(*count))
}

/// `y / 1s`, a quantity of time as its plain count of seconds; the unit marked, so a program's own `s` leaves it the unit
pub(crate) fn seconds_of(quantity: Node) -> Node {
	let one_second = Node::Key(Box::new(Node::int(1)), Op::Mul, Box::new(Node::Symbol(format!("{AFTER_NUMBER}s"))));
	Node::Key(Box::new(quantity), Op::Div, Box::new(one_second))
}

/// `km`, `m²`, `s^2`
fn is_unit_term(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(name) => names_unit(name.strip_prefix(AFTER_NUMBER).unwrap_or(name)) || shadowed_unit(name),
		Node::Key(base, _, power) if matches!(power.drop_meta(), Node::Empty | Node::Number(_)) => is_unit_term(base),
		_ => false,
	}
}

fn shadowed_unit(name: &str) -> bool {
	units().iter().any(|unit| unit.name == name) || unit_aliases().iter().any(|(alias, _)| *alias == name)
}

fn marked_unit(node: Node, shadowed: &ShadowedUnits) -> Node {
	match node {
		Node::Symbol(name) if shadowed.contains(&name) && units().iter().any(|unit| unit.name == name) => {
			Node::Symbol(format!("{AFTER_NUMBER}{name}"))
		}
		Node::Key(base, op, power) if matches!(power.drop_meta(), Node::Empty | Node::Number(_)) => {
			Node::Key(Box::new(marked_unit(*base, shadowed)), op, power)
		}
		Node::Meta { node, data } => Node::Meta { node: Box::new(marked_unit(*node, shadowed)), data },
		Node::Key(left, op @ (Op::Mul | Op::Div), right) if is_unit_type(&left) => {
			Node::Key(Box::new(marked_unit(*left, shadowed)), op, Box::new(marked_unit(*right, shadowed)))
		}
		other => marked_after_numbers(other, shadowed),
	}
}

/// `run` with the unit words `program` defines as its names, not units: `g(x) := x + 1 m` keeps m a unit
fn shadowing<T>(program: &Node, run: impl FnOnce() -> T) -> T {
	crate::compiler::compile_context::scoped(|context| &mut context.shadowed_units, defined_unit_names(program), run)
}

/// A conversion target: a unit by its short name or its long name, `minute` or `minutes`
fn target_unit(node: &Node) -> Option<&'static Unit> {
	let Node::Symbol(name) = node.drop_meta() else {
		return None;
	};
	let singular = name.strip_suffix('s').unwrap_or(name);
	unit_named(name).or_else(|| {
		long_names()
			.iter()
			.find(|(long, _)| *long == singular || *long == name)
			.and_then(|(_, short)| unit_named(short))
	})
}

/// The unit words as the units passes read them: aliases expanded (`60 mph`), the whole unit after `as` (`q as km/h`)
pub fn lower_unit_words(program: Node) -> Node {
	let program = with_units_after_numbers(lower_unit_aliases(with_run_time_unit_tables(program)));
	// each cast word where the program leaves it free: a function's own `unit` is its variable
	let program = QUANTITY_CAST_WORDS.into_iter().fold(program, |program, word| {
		crate::program_names::rewritten_where_free(program, word, &|free| with_quantity_casts(free, word))
	});
	shadowing(&program.clone(), || with_whole_conversion_units(program))
}

/// lib/units.warp's `unit_definitions = data {…}` is read once by definitions.rs; at run time it stands for the tables
/// its Quantity class reads (`unit_names = ["mm", …]`), so the block is their one source (card lib-units)
pub(crate) fn with_run_time_unit_tables(program: Node) -> Node {
	let is_block = |statement: &Node| matches!(statement.drop_meta(), Node::Key(name, Op::Assign, _) if name.drop_meta() == &Node::Symbol(definitions::BLOCK_NAME.to_string()));
	let tables = || statements_of(crate::warp_parser::parse(&definitions::run_time_tables()));
	match program {
		Node::List(items, Bracket::None, separator)
			if separator.separates_statements() && items.iter().any(is_block) =>
		{
			let items = items.into_iter().flat_map(|item| if is_block(&item) { tables() } else { vec![item] });
			Node::List(items.collect(), Bracket::None, separator)
		}
		program if is_block(&program) => Node::List(tables(), Bracket::None, Separator::Newline),
		program => program,
	}
}

/// The program's statements, a line's `;` statements among them (`unit hand := 4inch; x = 2 hand⏎x`)
fn statements_of(node: Node) -> Vec<Node> {
	match node {
		Node::List(items, Bracket::None, separator) if separator.separates_statements() => {
			items.into_iter().flat_map(statements_of).collect()
		}
		node => vec![node],
	}
}

/// The words starting a program's unit lines, each with how a refused one is written right: `unit furlong := 201.168 m`
const SI_WORD: &str = "SI";
const PREFIX_LINE_WORD: &str = "prefix";
const NAME_LINE_WORD: &str = "name";
const POWER_OF_TEN: &str = "10^";
const QUANTITY_LINE_WORD: &str = "quantity";
/// `dimension`, the old word of a quantity line: a quantity, a tensor's axis, its rank or its shape (decision
/// quantity-naming)
const DIMENSION_WORD: &str = "dimension";
const DIMENSION_AMBIGUOUS: &str = "dimension: quantity, axis, rank or shape? a unit line names a kind of quantity";
const UNIT_LINE_WORDS: [(&str, &str); 3] = [
	("unit", "the right side is no amount of a known unit (`unit furlong := 201.168 m`)"),
	(
		PREFIX_LINE_WORD,
		"a prefix is a power of ten that keeps every unit whole nanometers, nanoseconds … (`prefix k := 10^3`)",
	),
	(NAME_LINE_WORD, "the right side is no unit (`name furlong := fur`)"),
];

/// The program without its unit lines, which definitions.rs read before it was parsed (program_unit_lines); Err for a
/// line that defines no unit (`unit x := 3 foo`: foo is no unit)
pub fn without_unit_lines(program: Node) -> Result<Node, Node> {
	let unit_line = |statement: &Node| match statement.drop_meta() {
		Node::List(items, Bracket::None, Separator::Space) => match items.as_slice() {
			[word, definition, ..] => {
				let word =
					UNIT_LINE_WORDS.iter().find(|(line_word, _)| word.drop_meta().symbol_name() == Some(line_word))?;
				match definition.drop_meta() {
					// `name p := p.first` is a function of a parameter p: a line is a unit line where its right side reads as one
					Node::Key(_, Op::Define, right) if !reads_as_unit_line(word.0, right) => None,
					// `unit °C as K := …` names °C
					Node::Key(name, Op::Define, _) => match name.drop_meta() {
						Node::Key(name, Op::As, _) => Some((word, name.drop_meta().serialize())),
						name => Some((word, name.serialize())),
					},
					_ => None,
				}
			}
			_ => None,
		},
		_ => None,
	};
	let statements = statements_of(program.clone());
	if let Some((line, word)) =
		statements.iter().find_map(|statement| Some((statement, quantity_line_word(statement)?)))
	{
		if word == DIMENSION_WORD {
			crate::normalize::hint(DIMENSION_WORD, QUANTITY_LINE_WORD, DIMENSION_AMBIGUOUS);
		}
		return Err(error(&format!(
			"{}: quantities (dimensions) are lib/units.warp's, a program defines units, prefixes and names",
			line.serialize().trim()
		)));
	}
	if !statements.iter().any(|statement| unit_line(statement).is_some()) {
		return Ok(program);
	}
	if let Some(((word, fix), unknown)) =
		statements.iter().filter_map(unit_line).find(|(word, name)| !definitions::defines(word.0, name))
	{
		return Err(error(&format!("{word} {unknown}: {fix}")));
	}
	let kept: Vec<Node> = statements.into_iter().filter(|statement| unit_line(statement).is_none()).collect();
	Ok(Node::List(kept, Bracket::None, Separator::Newline))
}

/// The right side of a program's `unit`, `prefix` or `name` line as one reads: an amount of a unit (a wrong one is
/// reported), a power of ten `10^3`, a unit word; anything else leaves the line a definition (`prefix s := s#1`)
fn reads_as_unit_line(line_word: &str, right: &Node) -> bool {
	match line_word {
		PREFIX_LINE_WORD => {
			let written: String = right.serialize().split_whitespace().collect();
			written.strip_prefix(POWER_OF_TEN).is_some_and(|exponent| exponent.parse::<i32>().is_ok())
		}
		NAME_LINE_WORD => right.drop_meta().symbol_name().is_some_and(names_unit),
		_ => true,
	}
}

/// `SI quantity length meter` (or the old `SI dimension …`): a line only lib/units.warp writes; its second word
fn quantity_line_word(statement: &Node) -> Option<&str> {
	let Node::List(items, Bracket::None, Separator::Space) = statement.drop_meta() else {
		return None;
	};
	let mut words = items.iter().take(2).map(|item| item.drop_meta().symbol_name());
	match (words.next()??, words.next()??) {
		(SI_WORD, word @ (QUANTITY_LINE_WORD | DIMENSION_WORD)) => Some(word),
		_ => None,
	}
}

/// `x as quantity`, and `x as unit` with a got-it note: the run-time quantity(x) (card unit-possible)
const QUANTITY_CAST_WORDS: [&str; 2] = [RUN_TIME_QUANTITY_WORD, "unit"];

fn with_quantity_casts(node: Node, cast_word: &str) -> Node {
	let node = node.map_children(|child| with_quantity_casts(child, cast_word));
	let Node::Key(value, Op::As, target) = node.drop_meta() else {
		return node;
	};
	let Some(word) = target.drop_meta().symbol_name().filter(|word| *word == cast_word) else {
		return node;
	};
	if word != RUN_TIME_QUANTITY_WORD {
		crate::normalize::set_position_of(&node);
		crate::errors::diagnostic::note_alias(&format!("as {word}"), &format!("as {RUN_TIME_QUANTITY_WORD}"));
	}
	as_run_time_quantity(value).unwrap_or_else(|| {
		Node::List(
			vec![Node::Symbol(RUN_TIME_QUANTITY_WORD.to_string()), (**value).clone()],
			Bracket::Round,
			Separator::None,
		)
	})
}

/// `60 mi/h as km/h` parses `(60 mi/h as km)/h`: the unit after `as` regrouped whole, as after `in`, when it is one
fn with_whole_conversion_units(node: Node) -> Node {
	let node = match with_written_amount(node.map_children(with_whole_conversion_units)) {
		Node::Key(conversion, op @ (Op::Div | Op::Mul), rest) => match conversion.drop_meta() {
			Node::Key(quantity, Op::As, unit)
				if unit_expression(&Node::Key(unit.clone(), op, rest.clone())).is_some() =>
			{
				Node::Key(quantity.clone(), Op::As, Box::new(Node::Key(unit.clone(), op, rest)))
			}
			_ => Node::Key(conversion, op, rest),
		},
		other => other,
	};
	with_converted_last_operand(node)
}

/// A quantity as it may be written, in the form `amount unit` the units passes read: `1 mile` is `1 mi`,
/// `3km+10m == 3010 meters` gives the unit to the last amount, `(23/18)m/s` (a fraction amount, as one shows) is
/// `(23/18) * (m/s)`, and `6 m² as cm²` (parsed `(6 m² as cm)²`) converts to cm²
fn with_written_amount(node: Node) -> Node {
	match node {
		Node::List(items, Bracket::None, Separator::Space) if items.len() == 2 => {
			let (amount, unit) = (&items[0], &items[1]);
			let short_name = target_unit(unit).map(|unit| Node::Symbol(unit.name.to_string()));
			match (amount.drop_meta(), short_name) {
				(Node::Key(..), Some(short_name)) => with_unit(amount, &short_name),
				(_, Some(short_name)) => Node::List(vec![amount.clone(), short_name], Bracket::None, Separator::Space),
				(Node::List(_, Bracket::Round, _), None) if unit_expression(unit).is_some() => Node::Key(
					Box::new(amount.clone()),
					Op::Mul,
					Box::new(Node::List(vec![unit.clone()], Bracket::Round, Separator::None)),
				),
				_ => Node::List(items, Bracket::None, Separator::Space),
			}
		}
		Node::Key(conversion, Op::Pow, exponent) if powered_conversion(&conversion, &exponent).is_some() => {
			let Node::Key(quantity, Op::As, unit) = conversion.drop_meta().clone() else {
				unreachable!("a powered conversion is an `as`")
			};
			Node::Key(quantity, Op::As, Box::new(Node::Key(unit, Op::Pow, exponent)))
		}
		other => other,
	}
}

/// `"a: " + x as km` parses `("a: " + x) as km`: a text has no unit to convert, so `as` takes the operand after the
/// last `+` (card print-km)
fn with_converted_last_operand(node: Node) -> Node {
	match node {
		Node::Key(sum, Op::As, unit) if unit_expression(&unit).is_some() => match sum.drop_meta() {
			Node::Key(text, Op::Add, last) if joins_text(text) => {
				Node::Key(text.clone(), Op::Add, Box::new(Node::Key(last.clone(), Op::As, unit)))
			}
			_ => Node::Key(sum, Op::As, unit),
		},
		other => other,
	}
}

/// `"a: "`, `"a: " + x + " or "`: a text, or a sum with a text in it, which every operand joins as its text
pub(crate) fn joins_text(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Text(_) => true,
		Node::Key(left, Op::Add, right) => joins_text(left) || joins_text(right),
		_ => false,
	}
}

/// `60 mph` as `60 mi/h`: each unit alias the program does not define read as its unit expression (not a field name:
/// `r.mph`, `{mph: 3}`)
fn lower_unit_aliases(program: Node) -> Node {
	let aliases: Vec<(&str, &str)> =
		unit_aliases().iter().copied().filter(|(alias, _)| mentions_word(&program, alias)).collect();
	if aliases.is_empty() {
		return program;
	}
	let defined = defined_unit_names(&program);
	let aliases: Vec<(&str, &str)> = aliases.into_iter().filter(|(alias, _)| !defined.contains(*alias)).collect();
	with_unit_aliases(program, &aliases, &defined)
}

/// An alias's expansion: unit words even where the program defines their names (`s = …; 440Hz` is 440/s)
fn marked_expansion(node: Node, defined: &std::collections::HashSet<String>) -> Node {
	match node {
		Node::Symbol(name) if defined.contains(&name) => Node::Symbol(format!("{AFTER_NUMBER}{name}")),
		other => other.map_children(|child| marked_expansion(child, defined)),
	}
}

fn with_unit_aliases(node: Node, aliases: &[(&str, &str)], defined: &std::collections::HashSet<String>) -> Node {
	let expansion = |unit: &Node| match unit.drop_meta() {
		Node::Symbol(name) => aliases
			.iter()
			.find(|(alias, _)| alias == name)
			.map(|(_, expression)| marked_expansion(crate::warp_parser::parse(expression), defined)),
		_ => None,
	};
	match node {
		// `60 mph`, `60*mph`: the amount joins the first unit, `60 mi / h`, as a written `60 mi/h` reads
		Node::List(items, Bracket::None, Separator::Space) if items.len() == 2 && expansion(&items[1]).is_some() => {
			let [amount, unit] = <[Node; 2]>::try_from(items).expect("two items");
			times_first_unit(with_unit_aliases(amount, aliases, defined), expansion(&unit).expect("guarded"))
		}
		Node::Key(amount, Op::Mul, unit) if expansion(&unit).is_some() => {
			times_first_unit(with_unit_aliases(*amount, aliases, defined), expansion(&unit).expect("guarded"))
		}
		Node::Symbol(name) => expansion(&Node::Symbol(name.clone())).unwrap_or(Node::Symbol(name)),
		// a field's type, `speed: mph`
		Node::Type { name, body } if body.is_empty_node() && expansion(&name).is_some() => {
			expansion(&name).expect("guarded")
		}
		Node::Type { name, body } => Node::Type { name, body: Box::new(with_unit_aliases(*body, aliases, defined)) },
		Node::Key(object, Op::Dot, field) => {
			Node::Key(Box::new(with_unit_aliases(*object, aliases, defined)), Op::Dot, field)
		}
		Node::Key(field, Op::Colon, value) => {
			Node::Key(field, Op::Colon, Box::new(with_unit_aliases(*value, aliases, defined)))
		}
		other => other.map_children(|child| with_unit_aliases(child, aliases, defined)),
	}
}

/// Whether the word occurs anywhere, class bodies and field types included
fn mentions_word(node: &Node, word: &str) -> bool {
	match node.drop_meta() {
		Node::Symbol(name) => name == word,
		Node::Type { name, body } => mentions_word(name, word) || mentions_word(body, word),
		other => other.parts().into_iter().any(|child| mentions_word(child, word)),
	}
}

fn times_first_unit(amount: Node, expression: Node) -> Node {
	match expression.drop_meta() {
		Node::Key(first, op @ (Op::Mul | Op::Div), rest) => {
			Node::Key(Box::new(times_first_unit(amount, *first.clone())), *op, rest.clone())
		}
		_ => Node::Key(Box::new(amount), Op::Mul, Box::new(expression)),
	}
}

/// A unit word's base dimension and how many of the dimension's smallest steps it is (`km`: Length, 10_000_000)
pub fn dimension_and_factor(name: &str) -> Option<(String, i64)> {
	unit_named(name).map(|unit| (format!("{:?}", unit.dimension), unit.factor))
}

/// `h` as its unit's long name `hour` (the unit word where it has none); none when `program` defines the word itself
pub fn unit_long_name(name: &str, program: &Node) -> Option<&'static str> {
	let unit = shadowing(program, || unit_named(name))?;
	let long_name = long_names().iter().find(|(_, short)| *short == unit.name);
	Some(long_name.map_or(unit.name, |(long, _)| *long))
}

pub fn is_unit(name: &str) -> bool {
	unit_named(name).is_some()
}

/// A unit or a unit alias: the parser binds it to the amount before it, `60 mph` is `60*mph`
pub fn names_unit(name: &str) -> bool {
	is_unit(name) || unit_aliases().iter().any(|(alias, _)| *alias == name)
}

/// lib/units.warp's class of quantities whose unit is known only at run time, and the word making one
pub const RUN_TIME_QUANTITY: &str = "Quantity";
const RUN_TIME_QUANTITY_WORD: &str = "quantity";

/// A unit written in the program (`1 m`, `5 m/s`, `6 m²`) as the run-time `quantity(1, "m")`, for where it meets one
/// (card units-mixed)
pub fn as_run_time_quantity(node: &Node) -> Option<Node> {
	let (amount, unit) = unit_literal(node)?;
	Some(run_time_quantity(amount, unit))
}

/// The amount and the unit text of `quantity(amount, "unit")`
pub fn run_time_quantity_parts(node: &Node) -> Option<(&Node, &str)> {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) => match items.as_slice() {
			[word, amount, unit] if word.drop_meta().name() == RUN_TIME_QUANTITY_WORD => match unit.drop_meta() {
				Node::Text(unit) => Some((amount, unit.as_str())),
				_ => None,
			},
			_ => None,
		},
		_ => None,
	}
}

fn run_time_quantity(amount: Node, unit: String) -> Node {
	Node::List(
		vec![Node::Symbol(RUN_TIME_QUANTITY_WORD.to_string()), amount, Node::Text(unit)],
		Bracket::Round,
		Separator::None,
	)
}

/// `5 m ± 1 cm` as the run-time `quantity(5 ± 1/100, "m")`: its amount an interval in the unit of the value (card
/// plus-minus-units)
pub fn lower_run_time_tolerances(program: Node) -> Node {
	shadowing(&program.clone(), || match has_unit_tolerance(&program) {
		true => with_run_time_tolerances(program),
		false => program,
	})
}

fn has_unit_tolerance(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(value, Op::PlusMinus, spread) if unit_literal(value).is_some() || unit_literal(spread).is_some() => {
			true
		}
		other => other.parts().into_iter().any(has_unit_tolerance),
	}
}

fn with_run_time_tolerances(node: Node) -> Node {
	match node {
		Node::Key(value, Op::PlusMinus, spread)
			if unit_literal(&value).is_some() || unit_literal(&spread).is_some() =>
		{
			run_time_tolerance(&value, &spread).unwrap_or_else(|message| error(&message))
		}
		other => other.map_children(with_run_time_tolerances),
	}
}

/// `5 m ± 1 cm` is `quantity(5 ± 1/100, "m")`, `5 m ± 1` counts the 1 in m, `5 ± 1 cm` the 5 in cm
fn run_time_tolerance(value: &Node, spread: &Node) -> Result<Node, String> {
	let interval = |amount: Node, spread: Node| Node::Key(Box::new(amount), Op::PlusMinus, Box::new(spread));
	let (amount, unit) = match (unit_literal(value), unit_literal(spread)) {
		// `5 m ± 1 cm`: the spread counts in the value's unit, value·spread/value in SI amounts
		(Some((amount, unit)), Some(_)) => {
			let si = |quantity: &Node| match static_units::constant(quantity) {
				Ok(Some((si_amount, signature))) => Ok((static_units::exact_amount(&si_amount), signature)),
				_ => Err(format!("a tolerance applies to numbers and quantities: {}", quantity.serialize())),
			};
			let ((value_si, value_signature), (spread_si, spread_signature)) = (si(value)?, si(spread)?);
			if value_signature != spread_signature {
				return Err("incompatible units".to_string());
			}
			let spread_in_unit = match (static_units::exact_amount(&amount), value_si, spread_si) {
				(Some(amount), Some(value_si), Some(spread_si)) => {
					value_si.inverse().map(|per_value| amount.mul(&spread_si).mul(&per_value))
				}
				_ => None,
			}
			.ok_or_else(|| {
				format!("{} ± {}: write the spread in the value's unit", value.serialize(), spread.serialize())
			})?;
			(interval(amount, quotient_node(&spread_in_unit)), unit)
		}
		(Some((amount, unit)), None) => (interval(amount, spread.clone()), unit),
		(None, Some((amount, unit))) => (interval(value.clone(), amount), unit),
		(None, None) => unreachable!("guarded by has_unit_tolerance"),
	};
	Ok(run_time_quantity(amount, unit))
}

/// `5 m/s` as its amount and unit text: a number times units, then more units multiplied or divided
fn unit_literal(node: &Node) -> Option<(Node, String)> {
	match node.ungrouped() {
		Node::Key(amount, Op::Mul, unit) if matches!(amount.drop_meta(), Node::Number(_)) => {
			Some((amount.drop_meta().clone(), unit_text(unit)?))
		}
		Node::Key(literal, op @ (Op::Mul | Op::Div), unit) => {
			let (amount, written) = unit_literal(literal)?;
			let joint = if *op == Op::Mul { "·" } else { "/" };
			Some((amount, format!("{written}{joint}{}", unit_text(unit)?)))
		}
		_ => None,
	}
}

/// `m`, `m²`: a unit's name, to a power
fn unit_text(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Symbol(name) if is_unit(name) => Some(name.clone()),
		Node::Key(base, Op::Pow, exponent) => {
			Some(format!("{}{}", unit_text(base)?, superscript(i64::from(whole_exponent(exponent)?))))
		}
		_ => None,
	}
}

/// `meters`, `kilogram`: a long name of a quantity's unit; durations (`2 minutes`) belong to the time module
pub fn is_long_unit_name(name: &str) -> bool {
	!is_unit(name)
		&& target_unit(&Node::Symbol(name.to_string())).is_some_and(|unit| unit.dimension != Dimension::duration())
}

/// A unit to a power within a quantity: km¹, h⁻¹, m²
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Factor {
	unit: &'static Unit,
	power: i32,
}

/// An exact amount (a whole number or a fraction: `3/2 m`, `23/18 m/s`) of a product of units to powers: `3 m`, `6 m²`,
/// `6 m·kg`, `5 km/h` (km·h⁻¹), `1 kg·m/s²`.
/// A quantity equals the bare number of its own unit: `3km+10m == 3010`
#[derive(Clone, Debug, PartialEq)]
pub struct Quantity {
	amount: Rational,
	factors: Vec<Factor>,
}

/// Between the units of a product: `m·kg`
const UNIT_PRODUCT: &str = "·";

/// `m·kg/s²`: the units with a positive power, then those with a negative one after a slash
fn units_text(factors: &[Factor]) -> String {
	let side = |positive: bool| {
		let units = factors.iter().filter(|factor| (factor.power > 0) == positive).map(|factor| {
			match factor.power.unsigned_abs() {
				1 => factor.unit.name.to_string(),
				power => format!("{}{}", factor.unit.name, crate::extensions::reals::superscript(power.into())),
			}
		});
		units.collect::<Vec<String>>().join(UNIT_PRODUCT)
	};
	match (side(true), side(false)) {
		(above, below) if below.is_empty() => above,
		(above, below) if above.is_empty() => format!("1/{below}"),
		(above, below) => format!("{above}/{below}"),
	}
}

/// An amount as it reads back before a unit: 3, 9.81, and a fraction without a finite decimal in parentheses, (23/18)
pub(crate) fn amount_text(amount: &Rational) -> String {
	amount.decimal_text().unwrap_or_else(|| format!("({amount})"))
}

/// The units as they follow an amount, right after it (user 2026-10-08: no space): m, m/s², and /m for 1/m (3/m)
pub(crate) fn unit_suffix(factors: &[Factor]) -> String {
	let units = units_text(factors);
	match units.strip_prefix("1/") {
		Some(below) => format!("/{below}"),
		None => units,
	}
}

impl fmt::Display for Quantity {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}{}", amount_text(&self.amount), unit_suffix(&self.factors))
	}
}

/// A ± amount of units, a final unit field given `5 m ± 1 cm`: `5.000 ± 0.010m`; a Gaussian keeps its σ apart from the
/// unit, `12.00 ± 0.60σ m`, as lib/units.warp quantity_text shows it (card quantity-final)
#[derive(Clone, Debug, PartialEq)]
pub struct UncertainQuantity {
	amount: crate::numbers::uncertain::Uncertain,
	factors: Vec<Factor>,
}

impl UncertainQuantity {
	pub(crate) fn new(amount: crate::numbers::uncertain::Uncertain, factors: &[Factor]) -> UncertainQuantity {
		UncertainQuantity { amount, factors: factors.to_vec() }
	}
}

impl fmt::Display for UncertainQuantity {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let apart = if self.amount.gaussian { " " } else { "" };
		write!(f, "{}{apart}{}", self.amount, unit_suffix(&self.factors))
	}
}

impl Quantity {
	/// Is the amount the whole number n: a quantity equals the bare number of its own unit (`3km+10m == 3010`)
	pub fn is_amount(&self, n: i64) -> bool {
		self.amount == Rational::integer(n)
	}
}

/// Dimension → power, the same for `km/h` and `m/s`; two quantities add or compare only with equal signatures
fn signature(factors: &[Factor]) -> Vec<(Dimension, i32)> {
	factors.iter().fold(vec![], |powers, factor| added(&powers, factor.unit.dimension, factor.power))
}

/// `powers` with `power` more of `dimension`, in the order signatures keep
fn added(powers: &[(Dimension, i32)], dimension: Dimension, power: i32) -> Vec<(Dimension, i32)> {
	let mut added = powers.to_vec();
	match added.iter_mut().find(|(known, _)| *known == dimension) {
		Some((_, sum)) => *sum += power,
		None => added.push((dimension, power)),
	}
	added.retain(|(_, power)| *power != 0);
	added.sort_by_key(|(dimension, _)| dimension.0);
	added
}

/// The text of a unit result held in a `Node::Data`, None for any other data
pub fn describe(data: &DataValue) -> Option<String> {
	if let Some(quantity) = data.downcast_ref::<Quantity>() {
		return Some(quantity.to_string());
	}
	if let Some(uncertain) = data.downcast_ref::<crate::numbers::uncertain::Uncertain>() {
		return Some(uncertain.to_string());
	}
	if let Some(uncertain) = data.downcast_ref::<UncertainQuantity>() {
		return Some(uncertain.to_string());
	}
	if let Some(duration) = data.downcast_ref::<crate::time::Duration>() {
		return Some(duration.to_string());
	}
	data.downcast_ref::<crate::time::Time>().map(|time| time.to_string())
}

/// The unit of a bare number of sleep (decision sleep-seconds)
const SECOND: &str = "s";
/// what the host word sleep takes per second of a run-time number
const MILLISECONDS_PER_SECOND: i64 = 1000;

/// `sleep(1000 ms)`, `sleep 1 s`, `sleep(2 seconds)`, `sleep 2`: a constant duration is its milliseconds, what the host
/// word takes; a run-time number counts seconds as a bare one does, so `sleep(t)` sleeps `t * 1000` milliseconds
/// (async agent, 2026-10-06: quantities do not reach run time yet, notes/units_runtime.md)
pub fn lower_sleep_durations(program: Node) -> Node {
	// a duration whose unit word is one of the program's names (`s = 1; sleep 5 s`) stays as written; `m = 2` leaves
	// `sleep 5 ms` a duration (card sleep-unit)
	fn lower(node: Node, shadowed: &std::collections::HashSet<String>) -> Node {
		match node {
			// the word itself: `(sleep 300); 1` is a statement list whose first item merely starts with sleep
			Node::List(items, bracket, separator)
				if items.len() >= 2
					&& matches!(items[0].drop_meta(), Node::Symbol(word) if word == crate::host::SLEEP) =>
			{
				let duration = match &items[1..] {
					[single] => single.clone(),
					several => Node::List(several.to_vec(), Bracket::None, Separator::Space),
				};
				let shadows = duration.mentions_where(|name| shadowed.contains(name));
				let bare = matches!(duration.drop_meta(), Node::Number(_));
				if bare {
					if let Err(error) = warn_bare_duration(&duration) {
						return error;
					}
				}
				let duration = match bare {
					true => Node::List(vec![duration, Node::Symbol(SECOND.into())], Bracket::None, Separator::Space),
					false => duration,
				};
				let sleeping = |milliseconds: Node| {
					Node::List(vec![items[0].clone(), milliseconds], Bracket::Round, Separator::None)
				};
				match (milliseconds(&duration).filter(|_| !shadows), &items[1..]) {
					(Some(amount), _) => sleeping(Node::int(amount)),
					// a number at run time counts seconds too (decision sleep-seconds): `f(t) := sleep(t)`
					(None, [seconds]) => sleeping(Node::Key(
						Box::new(lower(seconds.clone(), shadowed)),
						Op::Mul,
						Box::new(Node::int(MILLISECONDS_PER_SECOND)),
					)),
					(None, _) => {
						Node::List(items.into_iter().map(|item| lower(item, shadowed)).collect(), bracket, separator)
					}
				}
			}
			other => other.map_children(|child| lower(child, shadowed)),
		}
	}
	let shadowed = defined_unit_names(&program);
	lower(program, &shadowed)
}

/// Comparisons of quantities where the rest of the program runs (card time-day-value): a comparison of constant
/// quantities, also of a main-level variable assigned one once (`x = 2 km; if x > 1500 m : …`), is its answer 1 or 0,
/// and such a variable no other statement reads is dropped; the word `time` of a program that names nothing so is the
/// local time of day of now, compared with a constant duration in milliseconds: `now < 24h` is
/// `system·time of day < 86400000` (lowering/signals/system_values.rs reads it, so `whenever now > 18h {…}` listens too)
pub fn lower_quantity_comparisons(program: Node) -> Node {
	shadowing(&program.clone(), || quantity_comparisons_folded(program))
}

fn quantity_comparisons_folded(program: Node) -> Node {
	if !(needs_quantities(&program) || program.mentions_any(&[NOW_WORD, TIME_WORD])) {
		return program;
	}
	let bound = crate::program_names::Bindings::of(&program).bound();
	let constants = constant_quantities(&program);
	let clock = !bound.contains(NOW_WORD);
	let time_is_the_type = !bound.contains(TIME_WORD);
	let folded = fold_comparisons(program, &constants, clock, time_is_the_type);
	without_unread_constants(folded, &constants)
}

/// The main-level variables assigned a quantity once, and nowhere else: `x = 2 km`, each with its value
fn constant_quantities(program: &Node) -> Constants {
	let mut assignments: HashMap<String, usize> = HashMap::new();
	program.visit(&mut |part| {
		if let Node::Key(target, Op::Assign | Op::Define | Op::Arrow, _)
		| Node::Key(target, Op::AddAssign | Op::SubAssign | Op::MulAssign | Op::DivAssign, _) = part
		{
			crate::variable_signals::symbols(target)
				.into_iter()
				.for_each(|name| *assignments.entry(name).or_default() += 1);
		}
	});
	let mut constants = Constants::new();
	for statement in crate::variable_signals::main_statements(program).0 {
		let Node::Key(target, Op::Assign, value) = statement.drop_meta() else {
			continue;
		};
		let Node::Symbol(name) = target.drop_meta() else {
			continue;
		};
		let value = with_constants(value, &constants);
		if assignments.get(name) == Some(&1) && needs_quantities(&value) {
			if let Ok(Some((_, signature))) = static_units::constant(&value) {
				if !signature.is_empty() {
					constants.insert(name.clone(), value);
				}
			}
		}
	}
	constants
}

/// `x > 1500 m` of `x = 2 km`: the expression with the constants' values in place of their names
fn with_constants(node: &Node, constants: &Constants) -> Node {
	constants.iter().fold(node.clone(), |node, (name, value)| {
		let grouped = Node::List(vec![value.clone()], Bracket::Round, Separator::None);
		crate::lowering::library_words::substitute(node, name, &grouped)
	})
}

fn fold_comparisons(node: Node, constants: &Constants, clock: bool, time_is_the_type: bool) -> Node {
	let Node::Key(left, op, right) = node.drop_meta() else {
		return node.map_children(|child| fold_comparisons(child, constants, clock, time_is_the_type));
	};
	if !op.is_comparison() {
		return node.map_children(|child| fold_comparisons(child, constants, clock, time_is_the_type));
	}
	let is_clock = |side: &Node| clock && side.is_symbol(NOW_WORD);
	let type_test = *op == Op::Eq && !crate::type_tests::is_equality_operand(right);
	if time_is_the_type && (left.is_symbol(TIME_WORD) || right.is_symbol(TIME_WORD) && !type_test) {
		let other = if left.is_symbol(TIME_WORD) { right } else { left };
		if let Some(amount) = milliseconds(&with_constants(other, constants)) {
			return error(&format!(
				"time is the type of times, not the clock: compare now, e.g. now {op} {}",
				written_milliseconds(amount)
			));
		}
	}
	if is_clock(left) || (is_clock(right) && !type_test) {
		let (duration, clock_left) = if is_clock(left) { (right, true) } else { (left, false) };
		return match clock_milliseconds(&with_constants(duration, constants)) {
			Ok(Some(amount)) => {
				let time_of_day = Node::Symbol(format!(
					"{}{}",
					crate::system_values::SYSTEM_PREFIX,
					warp_runtime::host_words::TIME_OF_DAY
				));
				let (left, right) =
					if clock_left { (time_of_day, Node::int(amount)) } else { (Node::int(amount), time_of_day) };
				Node::Key(Box::new(left), *op, Box::new(right))
			}
			Ok(None) => node,
			Err(message) => error(&message),
		};
	}
	// `1 h == 60 minutes`: a time unit beside a date or a duration of the time module is a duration, time::lower compares
	let reads_quantities = !crate::time::mentions_time(&node)
		&& (needs_quantities(&node) || constants.keys().any(|name| crate::warp_parser::mentions(&node, name)));
	match static_units::constant(&with_constants(&node, constants)) {
		Ok(Some((truth @ (Node::True | Node::False), _))) if reads_quantities => truth,
		Err(message) if reads_quantities => error(&message),
		_ => node.map_children(|child| fold_comparisons(child, constants, clock, time_is_the_type)),
	}
}

/// `18h`, `90min`, `500ms`: an amount of milliseconds in the largest unit it is whole in
fn written_milliseconds(amount: i64) -> String {
	const UNITS: [(i64, &str); 3] = [(3_600_000, "h"), (60_000, "min"), (1000, "s")];
	match UNITS.iter().find(|(size, _)| amount % size == 0) {
		Some((size, unit)) => format!("{}{unit}", amount / size),
		None => format!("{amount}ms"),
	}
}

/// What the time of day is compared with, in milliseconds; None when it is no constant (left as written, loud later)
fn clock_milliseconds(duration: &Node) -> Result<Option<i64>, String> {
	if let Some(amount) = milliseconds(duration) {
		return Ok(Some(amount));
	}
	match static_units::constant(duration) {
		Ok(Some((amount, signature))) if signature.is_empty() => {
			Err(format!("the time of day is a duration: compare it with one, e.g. now < {}h", amount.serialize()))
		}
		Ok(Some(_)) => Err(format!("DimensionError: the time of day is a duration, {} is none", duration.serialize())),
		_ => Ok(None),
	}
}

fn without_unread_constants(program: Node, constants: &Constants) -> Node {
	if constants.is_empty() {
		return program;
	}
	let (statements, bracket, separator) = crate::variable_signals::main_statements(&program);
	let assigns = |statement: &Node, name: &str| matches!(statement.drop_meta(), Node::Key(target, Op::Assign, _) if matches!(target.drop_meta(), Node::Symbol(assigned) if assigned == name));
	let unread = |name: &str| {
		statements.iter().all(|statement| assigns(statement, name) || !crate::warp_parser::mentions(statement, name))
	};
	let last = statements.len().saturating_sub(1);
	let kept: Vec<Node> = statements
		.iter()
		.enumerate()
		.filter(|(index, statement)| {
			*index == last || !constants.keys().any(|name| assigns(statement, name) && unread(name))
		})
		.map(|(_, statement)| statement.clone())
		.collect();
	if kept.len() == statements.len() {
		return program;
	}
	Node::List(kept, bracket, separator)
}

/// User #17: a bare number of `sleep(1)` gets the warning that names the units; it counts seconds (decision
/// sleep-seconds), as in sh and Python; `use relaxed` and `use script` take it without the warning
fn warn_bare_duration(duration: &Node) -> Result<(), Node> {
	if crate::compiler::compile_context::read(|context| context.hints_muted) {
		return Ok(());
	}
	let amount = duration.serialize();
	let message = format!("sleep needs a unit: sleep {amount}s or sleep {amount}ms (a bare number is seconds)");
	crate::errors::diagnostic::report(&[crate::errors::diagnostic::Diagnostic::at(duration, message)])
}

/// A constant duration in milliseconds, to the nearest one: `1000 ms`, `2 s`, `1 min`, `2 seconds`, `16.7 ms` (17), a
/// duration of the time module; the timers and sleep count whole milliseconds
pub(crate) fn milliseconds(node: &Node) -> Option<i64> {
	if let Node::Data(data) = node.drop_meta() {
		return duration_milliseconds(data.downcast_ref::<crate::time::Duration>()?, "ms").ok().flatten();
	}
	match static_units::constant(&with_written_amount(node.clone())) {
		Ok(Some((seconds, signature))) if signature == [(Dimension::duration(), 1)] => {
			nearest(&static_units::exact_amount(&seconds)?.mul(&Rational::integer(MILLISECONDS_PER_SECOND)))
		}
		_ => None,
	}
}

fn needs_quantities(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(name) => unit_named(name).is_some(),
		// `3010 meters`: a long name counts after an amount
		Node::List(items, Bracket::None, _) if matches!(items.as_slice(), [_, unit] if target_unit(unit).is_some()) => {
			true
		}
		Node::Key(_, Op::PlusMinus, _) => true,
		other => other.parts().into_iter().any(needs_quantities),
	}
}

/// The unit words the program defines as variables, parameters or functions (`m = 2`, `f(s) := …`)
fn defined_unit_names(node: &Node) -> std::collections::HashSet<String> {
	let mut names = std::collections::HashSet::new();
	collect_defined_unit_names(node, &mut names);
	names
}

/// The program's functions, methods, classes and module aliases (`use list as l`): after a number their name is the
/// unit, never a value to multiply by (`h(d) := d + 2 h` adds two hours)
fn collect_callable_names(node: &Node, names: &mut std::collections::HashSet<String>) {
	if let Some(definition) = crate::lowering::function_head(node) {
		names.insert(definition.name.to_string());
	}
	match node.drop_meta() {
		Node::Type { name, .. } => {
			if let Node::Symbol(name) = name.drop_meta() {
				names.insert(name.clone());
			}
		}
		Node::List(words, _, _) => {
			if let [keyword, used] = words.as_slice() {
				if let Some((_, alias)) = crate::lowering::welcome_forms::aliased_module(keyword, used) {
					names.insert(alias);
				}
			}
		}
		_ => {}
	}
	node.drop_meta().parts().into_iter().for_each(|child| collect_callable_names(child, names));
}

fn collect_defined_unit_names(node: &Node, names: &mut std::collections::HashSet<String>) {
	let mut add_unit = |node: &Node| {
		if let Node::Symbol(name) = node.drop_meta() {
			if units().iter().any(|unit| unit.name == name) || unit_aliases().iter().any(|(alias, _)| alias == name) {
				names.insert(name.clone());
			}
		}
	};
	// `data($m + 0)`: a hole names a variable, also in the arm it binds for (card unit-hole-name)
	if let Some(code) = crate::lowering::blocks::quoted_data(node) {
		code.visit(&mut |part| {
			if let Some(name) = crate::lowering::switch::hole_name(part) {
				add_unit(&Node::Symbol(name.to_string()))
			}
		});
	}
	let mut add_parameters = |head: &Node| match head.drop_meta() {
		Node::List(items, _, _) => items.iter().for_each(|item| match item.drop_meta() {
			Node::Key(name, Op::Colon, _) => add_unit(name),
			_ => add_unit(item),
		}),
		other => add_unit(other),
	};
	// a function, method or class of the program named like a unit is that one (`class N{…}`, `second() := …`)
	if let Some(definition) = crate::lowering::function_head(node) {
		add_parameters(&Node::Symbol(definition.name.to_string()));
		definition.written_parameters.iter().for_each(&mut add_parameters);
	}
	match node.drop_meta() {
		// a field named like a unit stays the field's name only (`r.mph`), a method's name is the method
		Node::Type { name, body } => {
			add_parameters(name);
			let members = std::iter::once(body.as_ref()).chain(body.drop_meta().parts());
			members
				.filter_map(crate::lowering::function_head)
				.for_each(|method| add_parameters(&Node::Symbol(method.name.to_string())));
		}
		Node::Key(head, Op::Assign | Op::Define, _) if matches!(head.drop_meta(), Node::List(..)) => {
			add_parameters(head)
		}
		Node::Key(target, Op::Assign | Op::Define | Op::Colon, _) => add_parameters(target),
		Node::Key(parameters, Op::Arrow, _) => add_parameters(parameters),
		// `for g in 0 to 2 {…}`: a loop variable named like a unit is the variable (card loop-variable)
		Node::List(words, _, _) => {
			if let Some(variable) = crate::lowering::nodes::for_in_variable(words) {
				add_unit(&Node::Symbol(variable))
			}
			// `use list as l`: the program's name for a module is the module
			if let [keyword, used] = words.as_slice() {
				if let Some((_, alias)) = crate::lowering::welcome_forms::aliased_module(keyword, used) {
					add_unit(&Node::Symbol(alias))
				}
			}
		}
		_ => {}
	}
	node.drop_meta().parts().into_iter().for_each(|child| collect_defined_unit_names(child, names));
}

/// Main-level variables holding a constant quantity, and its value: `x = 2 km`
type Constants = HashMap<String, Node>;

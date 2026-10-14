//! Ranges: `a..b`, `a down to b`, combined range readings
use super::*;
use crate::words::IN_WORD;

/// `lhs op rhs`; a chained comparison `a<b<c` as `a<b and b<c` (`middle` is b), a range subscript `xs#(a..b)` as a slice,
/// `a down to b` as the reversed `b to a`
pub(crate) fn combined(lhs: Node, op: Op, written: &str, rhs: Node, middle: Option<Node>) -> Node {
	match (middle, hash_slice_bounds(&rhs)) {
		(Some(middle), _) if op.is_ordering() => {
			let next_comparison = Node::Key(Box::new(middle), op, Box::new(rhs));
			Node::Key(Box::new(lhs), Op::And, Box::new(next_comparison))
		}
		(_, Some((start, end))) if op == Op::Hash => call(SLICE_WORD, vec![lhs, start, end]),
		// membership `x in xs` is kept as the list [x in xs]: `if "Z" in v {…}`, `found = x in xs`, `xs contains x`
		_ if op == Op::In => {
			let word = if written.ends_with(IN_WORD) { IN_WORD } else { written };
			Node::List(vec![lhs, Symbol(word.to_string()), rhs], Bracket::None, Separator::Space)
		}
		_ if op == Op::Pipe => piped(lhs, rhs),
		_ if matches!(op, Op::If | Op::While) => guarded_statement(lhs, op, written, rhs),
		// Java's and Scala's `import math.*`: a glob, nothing after the `*`, is the module whole
		_ if written == ".*" && matches!(rhs, Empty) => {
			crate::normalize::hint(&format!("{}.*", lhs.serialize()), &lhs.serialize(), "warp imports a module whole");
			lhs
		}
		// kept as written, lowering/collections/broadcasting.rs lower_element_wise_operators gives it its meaning
		_ if crate::operators::element_wise_operator(written).is_some() => {
			Node::List(vec![lhs, Symbol(written.to_string()), rhs], Bracket::None, Separator::Space)
		}
		_ if FLOOR_DIVISION_SPELLINGS.contains(&written) => call(FLOOR_QUOTIENT, vec![lhs, rhs]),
		_ if written == FLOOR_DIVISION_ASSIGNMENT => {
			let quotient = call(FLOOR_QUOTIENT, vec![lhs.clone(), rhs]);
			Node::Key(Box::new(lhs), Op::Assign, Box::new(quotient))
		}
		_ if op == Op::To && written.starts_with(DOWN_WORD) => {
			call(REVERSE_WORD, vec![Node::Key(Box::new(rhs), Op::To, Box::new(lhs))])
		}
		_ => Node::Key(Box::new(lhs), op, Box::new(rhs)),
	}
}

/// `n-1`, the end a Kotlin writer gives an inclusive `..`
pub(crate) fn is_minus_one(end: &Node) -> bool {
	match end.drop_meta() {
		Node::Key(_, Op::Sub, one) => matches!(one.drop_meta(), Node::Number(Number::Int(1))),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => is_minus_one(&items[0]),
		_ => false,
	}
}

impl WarpParser {
	/// Ranges whose end readers expect either way are Asks, by default read as warp does (exclusive):
	/// `a upto b` (wiki/range.md: excludes b) and the loop bound `for i in 0..n-1` (Kotlin's `..` includes n-1)
	pub(crate) fn range_reading(
		&self,
		op: Op,
		written: &str,
		end: &Node,
		line: usize,
		column: usize,
	) -> Result<Op, Node> {
		let end_text = crate::normalize::operand_text(end);
		let question = match written {
			UPTO => format!(
				"does `upto {end_text}` include {end_text}? (`..<` or `..` exclude it, `to` or `...` include it)"
			),
			EXCLUSIVE_DOTS if self.in_for_header && is_minus_one(end) => {
				format!("does the loop bound `..{end_text}` include {end_text}? (warp's `..` excludes it, Kotlin's includes it)")
			}
			_ => return Ok(op),
		};
		let topic = if written == UPTO { UPTO } else { KOTLIN_RANGE };
		let readings = vec![reading("exclusive", "..<"), reading("inclusive", "...")];
		let chosen = ask(&Ask::new(topic, question, readings, Fallback::Warning).written(written).at(line, column))?;
		Ok(if chosen == 0 { Op::Range } else { Op::To })
	}

	/// `10 down to 1`: the length of the words `down to` (Kotlin's `downTo`), the range counting down
	pub(crate) fn down_to_length(&self) -> Option<usize> {
		if !self.matches_keyword(DOWN_WORD) {
			return None;
		}
		let blanks = (DOWN_WORD.len()..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		(blanks > 0 && self.word_at(DOWN_WORD.len() + blanks) == TO_WORD)
			.then(|| DOWN_WORD.len() + blanks + TO_WORD.len())
	}
}

//! `now` compared with a part of a time compares that part of now (cards now-make, now-allow): with a duration its
//! local time of day (`now > 17h`, folded by units.rs), with a month name its month (`now >= October` is
//! `now.month >= 10`), with a date its local date (`now ≈ 2026-10-14`, `now < 2027-01-01`). `≈` asks for the same part,
//! `==` stays the whole value (an instant is no date). `time.now` is `now.time`, the local time of day, `22:27:46`
//! (a bare `time` stays the type, user decisions print-time and time-now). A program's own `now`, `time` or month
//! name wins.

use super::calendar::{parse_literal, Time};
use super::{TimeLiteral, NOW_WORD, TIME_WORD};
use crate::node::Node;
use crate::operators::Op;
use std::collections::HashSet;

const MONTH_NAMES: [&str; 12] = [
	"January",
	"February",
	"March",
	"April",
	"May",
	"June",
	"July",
	"August",
	"September",
	"October",
	"November",
	"December",
];
/// now's local date (calendar::Time::part)
const NOW_DATE: &str = "now.date";
const NOW_MONTH: &str = "now.month";
/// `time.now`, the current time of day (user decision time-now)
const NOW_TIME_OF_DAY: &str = "now.time";

pub fn lower(program: Node) -> Node {
	if !program.mentions(NOW_WORD) {
		return program;
	}
	let bound = crate::system_values::assigned_symbols(&program);
	if bound.contains(NOW_WORD) {
		return program;
	}
	rewrite(program, &bound)
}

fn rewrite(node: Node, bound: &HashSet<String>) -> Node {
	let node = node.map_children(|child| rewrite(child, bound));
	let Node::Key(left, op, right) = node.drop_meta() else {
		return node;
	};
	if *op == Op::Dot && left.is_symbol(TIME_WORD) && right.is_symbol(NOW_WORD) && !bound.contains(TIME_WORD) {
		return crate::warp_parser::parse(NOW_TIME_OF_DAY);
	}
	if !(op.is_ordering() || *op == Op::Similar) {
		return node;
	}
	let compared = match (left.is_symbol(NOW_WORD), right.is_symbol(NOW_WORD)) {
		(true, false) => part_compared(*op, right, bound),
		(false, true) => part_compared(mirrored(*op), left, bound),
		_ => None,
	};
	compared.unwrap_or(node)
}

/// now `op` the part: the same part of now compared with it, `≈` as `==`
fn part_compared(op: Op, part: &Node, bound: &HashSet<String>) -> Option<Node> {
	let exact = if op == Op::Similar { Op::Eq } else { op };
	let compare = |now_part: Node, value: Node| Some(Node::Key(Box::new(now_part), exact, Box::new(value)));
	if let Some(month) = month_number(part, bound) {
		return compare(crate::warp_parser::parse(NOW_MONTH), Node::int(month));
	}
	if !is_date(part) {
		return None;
	}
	compare(crate::warp_parser::parse(NOW_DATE), part.clone())
}

fn month_number(node: &Node, bound: &HashSet<String>) -> Option<i64> {
	let name = node.drop_meta().symbol_name().filter(|name| !bound.contains(*name))?;
	MONTH_NAMES.iter().position(|month| *month == name).map(|index| index as i64 + 1)
}

fn is_date(node: &Node) -> bool {
	let Node::Data(data) = node.drop_meta() else { return false };
	data.downcast_ref::<TimeLiteral>().is_some_and(|literal| matches!(parse_literal(&literal.0), Ok(Time::Date(_))))
}

/// The comparison with its sides swapped: `17h < now` is `now > 17h`
fn mirrored(op: Op) -> Op {
	match op {
		Op::Lt => Op::Gt,
		Op::Gt => Op::Lt,
		Op::Le => Op::Ge,
		Op::Ge => Op::Le,
		other => other,
	}
}

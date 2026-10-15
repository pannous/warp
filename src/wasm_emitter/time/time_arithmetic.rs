//! Arithmetic on times and durations at run time (card run-time-dates stage B2, notes/dates_at_run_time.md):
//! `d + 1 day`, `t - 90 minutes`, `a - b`, `1 month + 2 days`, `3 * 1 hour`, as calendar.rs `Time::add`, `since`,
//! `Duration::plus` and `times` do for constants. node_add, node_sub and node_mul hand over when an operand is a time.

use super::super::lists::list_ops::RETURNED_ERROR;
use super::super::time::time_zones::{REJECT, ZONED_NODE};
use super::super::time::times::{CIVIL_FROM_DAYS, DAYS_FROM_CIVIL, DAY_NANOS, TIME_FIELD, TIME_PARTS, ZONE_NAMES};
use super::super::WasmGcEmitter;
use crate::node::Node;
use crate::operators::Op;
use crate::time::calendar::{HOUR, LENGTHS_VARY, MINUTE, SECOND};
use crate::time::{TimeForm, MONTHS_SHIFT, OFFSET_SHIFT, TIME_FORM_BITS, TIME_FORM_MASK, ZONE_INDEX_MASK};
use crate::type_kinds::{Kind, KIND_BITS, KIND_MASK};
use wasm_encoder::*;
use Instruction as I;

/// time_arithmetic(a, b, operator) -> node; the operator is the index in TIME_OPERATORS
pub const TIME_ARITHMETIC: &str = "time_arithmetic";
/// duration_order(kind a, position a, kind b, position b) -> i32
pub(in crate::wasm_emitter) const DURATION_ORDER: &str = "duration_order";
const SHORTEST_MONTH_DAYS: i64 = 28;
const LONGEST_MONTH_DAYS: i64 = 31;
/// Daylight saving shifts a span by at most an hour
const HOUR_NANOS: i64 = HOUR * SECOND as i64;
/// The operators a time takes at run time
pub const TIME_OPERATORS: [Op; 3] = [Op::Add, Op::Sub, Op::Mul];

/// Where a duration keeps its days and months in the kind word: the info bits above the kind and form
const DAYS_SHIFT: i64 = KIND_BITS + TIME_FORM_BITS;
const MONTHS_IN_KIND: i64 = KIND_BITS + MONTHS_SHIFT;
const DAYS_MASK: i64 = 0xffff_ffff;
const DAYS_WIDTH: i64 = 32;
const MONTHS_PER_YEAR: i64 = 12;
const DURATION_KIND: i64 = ((TimeForm::Duration as i64) << KIND_BITS) | Kind::Time as i64;
const NO_TIME_OF_DAY: &str = "a date has no time of day: add that duration to a local time";
const NO_CALENDAR: &str = "an instant has no calendar: place it in a zone before adding months or days";
const CLOCK_WITHOUT_CALENDAR: &str = "a time of day has no calendar: add months or days to a local time";
const NO_DAY: &str = "the day does not exist in that month: use add(…, overflow: clamp) for the last day of the month";
const NO_ZONES: &str = "a zoned time at run time needs the time zone functions";
const MISMATCHED: &str = "cannot subtract a date, a local time, an instant and a zoned time from each other: no implicit conversion between kinds of time";
const NOT_ARITHMETIC: &str = "a time takes + or - a duration, a duration takes + or - a duration and * a whole number: nothing else is arithmetic on times";

const A: u32 = 0;
const B: u32 = 1;
const OPERATOR: u32 = 2;
// i64 locals after the parameters: the kinds and positions (a whole number's value) of the operands, then the duration
const KIND_A: u32 = 3;
const KIND_B: u32 = 4;
const POSITION_A: u32 = 5;
const POSITION_B: u32 = 6;
const MONTHS: u32 = 7;
const DAYS: u32 = 8;
const NANOS: u32 = 9;
const SIGN: u32 = 10;
const FORM: u32 = 11;
const SPARE: u32 = 12;
const YEAR: u32 = 13;
const MONTH: u32 = 14;
const DAY: u32 = 15;
const EPOCH_DAY: u32 = 16;
const DAY_PART_NANOS: u32 = 17;
const LAST_DAY: u32 = 18;
const ZONE: u32 = 19;
const EXACT_NANOS: u32 = 20;
const IS_ZONED: u32 = 21;
const LOCAL_COUNT: usize = 19;

type Emit = Vec<I<'static>>;

/// 1 when the kind word in `kind` is a time of a form for which `wanted` holds (a duration or not)
fn is_time_where(kind: u32, duration: bool) -> Emit {
	vec![
		I::LocalGet(kind),
		I::I64Const(KIND_MASK),
		I::I64And,
		I::I64Const(Kind::Time as i64),
		I::I64Eq,
		I::LocalGet(kind),
		I::I64Const(KIND_BITS),
		I::I64ShrU,
		I::I64Const(TIME_FORM_MASK),
		I::I64And,
		I::I64Const(TimeForm::Duration as i64),
		if duration { I::I64Eq } else { I::I64Ne },
		I::I32And,
	]
}

fn is_kind(kind: u32, wanted: Kind) -> Emit {
	vec![I::LocalGet(kind), I::I64Const(KIND_MASK), I::I64And, I::I64Const(wanted as i64), I::I64Eq]
}

fn is_operator(index: usize) -> Emit {
	vec![I::LocalGet(OPERATOR), I::I32Const(index as i32), I::I32Eq]
}

fn both(first: Emit, second: Emit) -> Emit {
	[first, second, vec![I::I32And]].concat()
}

/// The month, day or nanosecond count of the duration with that kind word and position
#[derive(Clone, Copy)]
enum Part {
	Months,
	Days,
	Nanos,
}

fn part_of(kind: u32, position: u32, part: Part) -> Emit {
	match part {
		Part::Months => vec![I::LocalGet(kind), I::I64Const(MONTHS_IN_KIND), I::I64ShrS],
		Part::Days => vec![
			I::LocalGet(kind),
			I::I64Const(DAYS_SHIFT),
			I::I64ShrU,
			I::I64Const(DAYS_WIDTH),
			I::I64Shl,
			I::I64Const(DAYS_WIDTH),
			I::I64ShrS,
		],
		Part::Nanos => vec![I::LocalGet(position)],
	}
}

const PARTS: [(Part, u32); 3] = [(Part::Months, MONTHS), (Part::Days, DAYS), (Part::Nanos, NANOS)];

impl WasmGcEmitter {
	pub(in crate::wasm_emitter) fn emit_time_arithmetic(&mut self) {
		if !self.uses_times() {
			return;
		}
		let node_ref = ValType::Ref(self.node_ref(false));
		self.runtime_function(
			TIME_ARITHMETIC,
			vec![node_ref, node_ref, ValType::I32],
			vec![node_ref],
			vec![ValType::I64; LOCAL_COUNT],
			|s, f| {
				s.emit_unpack_operands(f);
				// `3 * d` is `d * 3`, `d + t` is `t + d`
				let swap = [
					is_kind(KIND_A, Kind::Int),
					both(
						is_time_where(KIND_A, true),
						[is_time_where(KIND_B, false), is_operator(0), vec![I::I32And]].concat(),
					),
					vec![I::I32Or],
				]
				.concat();
				Self::emit_if(f, &swap, |f| {
					for (first, second) in [(KIND_A, KIND_B), (POSITION_A, POSITION_B)] {
						#[rustfmt::skip]
						Self::emit_list(f, &[
							I::LocalGet(first), I::LocalSet(SPARE),
							I::LocalGet(second), I::LocalSet(first),
							I::LocalGet(SPARE), I::LocalSet(second),
						]);
					}
				});
				Self::emit_if(f, &is_operator(2), |f| {
					Self::emit_if(
						f,
						&[both(is_time_where(KIND_A, true), is_kind(KIND_B, Kind::Int)), vec![I::I32Eqz]].concat(),
						|f| s.emit_time_failure(f, NOT_ARITHMETIC),
					);
					for (part, local) in PARTS {
						Self::emit_list(
							f,
							&[
								part_of(KIND_A, POSITION_A, part),
								vec![I::LocalGet(POSITION_B), I::I64Mul, I::LocalSet(local)],
							]
							.concat(),
						);
					}
					s.emit_return_duration(f);
				});
				// +1 for +, -1 for -
				Self::emit_list(
					f,
					&[I::I64Const(1), I::I64Const(-1), I::LocalGet(OPERATOR), I::I32Eqz, I::Select, I::LocalSet(SIGN)],
				);
				Self::emit_if(f, &both(is_time_where(KIND_A, true), is_time_where(KIND_B, true)), |f| {
					for (part, local) in PARTS {
						Self::emit_list(
							f,
							&[
								part_of(KIND_B, POSITION_B, part),
								vec![I::LocalGet(SIGN), I::I64Mul],
								part_of(KIND_A, POSITION_A, part),
								vec![I::I64Add, I::LocalSet(local)],
							]
							.concat(),
						);
					}
					s.emit_return_duration(f);
				});
				Self::emit_if(f, &both(is_time_where(KIND_A, false), is_time_where(KIND_B, true)), |f| {
					for (part, local) in PARTS {
						Self::emit_list(
							f,
							&[
								part_of(KIND_B, POSITION_B, part),
								vec![I::LocalGet(SIGN), I::I64Mul, I::LocalSet(local)],
							]
							.concat(),
						);
					}
					s.emit_shift_time(f);
				});
				Self::emit_if(
					f,
					&both(both(is_time_where(KIND_A, false), is_time_where(KIND_B, false)), is_operator(1)),
					|f| s.emit_time_difference(f),
				);
				s.emit_time_failure(f, NOT_ARITHMETIC);
			},
		);
		self.emit_duration_order();
	}

	/// duration_order(kind a, position a, kind b, position b) -> -1, 0 or 1, as Duration::compare: the difference is
	/// longer or shorter on every date, or the order depends on the date (an error)
	fn emit_duration_order(&mut self) {
		let (kind_a, position_a, kind_b, position_b) = (0, 1, 2, 3);
		let (months, days, nanos, fewest_days, most_days, shift) = (4, 5, 6, 7, 8, 9);
		self.runtime_function(
			DURATION_ORDER,
			vec![ValType::I64; 4],
			vec![ValType::I32],
			vec![ValType::I64; 6],
			|s, f| {
				for (part, local) in [(Part::Months, months), (Part::Days, days), (Part::Nanos, nanos)] {
					Self::emit_list(f, &part_of(kind_a, position_a, part));
					Self::emit_list(f, &part_of(kind_b, position_b, part));
					Self::emit_list(f, &[I::I64Sub, I::LocalSet(local)]);
				}
				#[rustfmt::skip]
			Self::emit_list(f, &[
				// an exact difference: the sign of its nanoseconds
				I::LocalGet(months), I::LocalGet(days), I::I64Or, I::I64Eqz, I::If(BlockType::Empty),
					I::LocalGet(nanos), I::I64Const(0), I::I64GtS, I::LocalGet(nanos), I::I64Const(0), I::I64LtS,
					I::I32Sub, I::Return,
				I::End,
				I::LocalGet(days), I::LocalGet(months), I::I64Const(SHORTEST_MONTH_DAYS), I::I64Mul, I::I64Add,
				I::LocalSet(fewest_days),
				I::LocalGet(days), I::LocalGet(months), I::I64Const(LONGEST_MONTH_DAYS), I::I64Mul, I::I64Add,
				I::LocalSet(most_days),
				// daylight saving moves an exact part against the calendar part by at most an hour; calendar parts alone keep the hour
				I::LocalGet(nanos), I::I64Const(0), I::I64Ne, I::I64ExtendI32U, I::I64Const(HOUR_NANOS), I::I64Mul, I::LocalSet(shift),
				// negative months take more days off the longer they are
				I::LocalGet(months), I::I64Const(0), I::I64LtS, I::If(BlockType::Empty),
					I::LocalGet(fewest_days), I::LocalGet(most_days), I::LocalSet(fewest_days), I::LocalSet(most_days),
				I::End,
				I::LocalGet(fewest_days), I::I64Const(DAY_NANOS), I::I64Mul, I::LocalGet(nanos), I::I64Add,
				I::LocalGet(shift), I::I64Sub, I::I64Const(0), I::I64GtS, I::If(BlockType::Empty),
					I::I32Const(1), I::Return,
				I::End,
				I::LocalGet(most_days), I::I64Const(DAY_NANOS), I::I64Mul, I::LocalGet(nanos), I::I64Add,
				I::LocalGet(shift), I::I64Add, I::I64Const(0), I::I64LtS, I::If(BlockType::Empty),
					I::I32Const(-1), I::Return,
				I::End,
			]);
				s.emit_time_failure(
					f,
					&format!("cannot order durations that compare differently on different dates ({LENGTHS_VARY})"),
				);
			},
		);
	}

	/// Required, not merely emitted: emit_all_functions builds node_add without times, whose refusals would mark the text
	/// constructors used (test_function_usage_tracking)
	pub(in crate::wasm_emitter) fn uses_times(&self) -> bool {
		self.ctx.required_functions.contains(TIME_FIELD)
	}

	/// The kind words and positions of both operands; a whole number's position is its value
	fn emit_unpack_operands(&mut self, f: &mut Function) {
		for (node, kind, position) in [(A, KIND_A, POSITION_A), (B, KIND_B, POSITION_B)] {
			self.emit_field(f, node, 0);
			Self::emit_list(f, &[I::LocalSet(kind)]);
			Self::emit_if(f, &is_kind(kind, Kind::Time), |f| {
				self.emit_time_position(f, node);
				f.instruction(&I::LocalSet(position));
			});
			Self::emit_if(f, &is_kind(kind, Kind::Int), |f| {
				Self::emit_list(f, &[I::LocalGet(node)]);
				self.call(f, "get_int_value");
				Self::emit_list(f, &[I::LocalSet(position)]);
			});
		}
	}

	/// `a + duration` where the duration's parts (already signed) are in MONTHS, DAYS and NANOS: calendar.rs `Time::add`
	fn emit_shift_time(&mut self, f: &mut Function) {
		let is_form = |form: TimeForm| vec![I::LocalGet(FORM), I::I64Const(form as i64), I::I64Eq];
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::LocalGet(KIND_A), I::I64Const(KIND_BITS), I::I64ShrU, I::I64Const(TIME_FORM_MASK), I::I64And,
			I::LocalSet(FORM),
		]);
		if self.ctx.required_functions.contains(ZONE_NAMES) {
			self.emit_zoned_start(f);
		} else {
			Self::emit_if(f, &is_form(TimeForm::Zoned), |f| self.emit_time_failure(f, NO_ZONES));
		}
		#[rustfmt::skip]
		let has_calendar_parts = || vec![
			I::LocalGet(MONTHS), I::I64Const(0), I::I64Ne, I::LocalGet(DAYS), I::I64Const(0), I::I64Ne, I::I32Or,
		];
		Self::emit_if(f, &is_form(TimeForm::Instant), |f| {
			Self::emit_if(f, &has_calendar_parts(), |f| self.emit_time_failure(f, NO_CALENDAR));
			Self::emit_list(f, &[I::LocalGet(POSITION_A), I::LocalGet(NANOS), I::I64Add, I::LocalSet(POSITION_A)]);
			self.emit_return_time(f, KIND_A, POSITION_A);
		});
		// a time of day goes round the clock: 23:00 + 2 hours is 01:00
		Self::emit_if(f, &is_form(TimeForm::Clock), |f| {
			Self::emit_if(f, &has_calendar_parts(), |f| self.emit_time_failure(f, CLOCK_WITHOUT_CALENDAR));
			Self::emit_list(f, &[I::LocalGet(POSITION_A), I::LocalGet(NANOS), I::I64Add, I::LocalSet(POSITION_A)]);
			Self::emit_floor_split(f, POSITION_A, DAY_NANOS, SPARE, POSITION_A);
			self.emit_return_time(f, KIND_A, POSITION_A);
		});
		Self::emit_if(
			f,
			&[is_form(TimeForm::Date), vec![I::LocalGet(NANOS), I::I64Const(0), I::I64Ne, I::I32And]].concat(),
			|f| self.emit_time_failure(f, NO_TIME_OF_DAY),
		);
		Self::emit_list(f, &[I::LocalGet(FORM), I::LocalGet(POSITION_A)]);
		self.call(f, TIME_PARTS);
		Self::emit_list(f, &[I::LocalSet(DAY_PART_NANOS), I::LocalSet(EPOCH_DAY), I::LocalGet(EPOCH_DAY)]);
		self.call(f, CIVIL_FROM_DAYS);
		Self::emit_list(f, &[I::LocalSet(DAY), I::LocalSet(MONTH), I::LocalSet(YEAR)]);
		// the month count first, then the days: 2024-01-31 + 1 month is rejected, + 1 month 2 days is 2024-02-31 + 2
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::LocalGet(YEAR), I::I64Const(MONTHS_PER_YEAR), I::I64Mul, I::LocalGet(MONTH), I::I64Add, I::I64Const(1),
			I::I64Sub, I::LocalGet(MONTHS), I::I64Add, I::LocalSet(SPARE),
		]);
		Self::emit_floor_split(f, SPARE, MONTHS_PER_YEAR, YEAR, MONTH);
		Self::emit_list(f, &[I::LocalGet(MONTH), I::I64Const(1), I::I64Add, I::LocalSet(MONTH)]);
		Self::emit_if(f, &[I::LocalGet(MONTHS), I::I64Const(0), I::I64Ne], |f| {
			#[rustfmt::skip]
			let first_of_next_month = [
				I::LocalGet(YEAR), I::LocalGet(MONTH), I::I64Const(MONTHS_PER_YEAR), I::I64Eq, I::I64ExtendI32U,
				I::I64Add, I::LocalGet(MONTH), I::I64Const(MONTHS_PER_YEAR), I::I64RemS, I::I64Const(1), I::I64Add,
				I::I64Const(1),
			];
			Self::emit_list(f, &first_of_next_month);
			self.call(f, DAYS_FROM_CIVIL);
			Self::emit_list(f, &[I::LocalGet(YEAR), I::LocalGet(MONTH), I::I64Const(1)]);
			self.call(f, DAYS_FROM_CIVIL);
			Self::emit_list(f, &[I::I64Sub, I::LocalSet(LAST_DAY)]);
			Self::emit_if(f, &[I::LocalGet(DAY), I::LocalGet(LAST_DAY), I::I64GtS], |f| {
				self.emit_time_failure(f, NO_DAY)
			});
		});
		Self::emit_list(f, &[I::LocalGet(YEAR), I::LocalGet(MONTH), I::LocalGet(DAY)]);
		self.call(f, DAYS_FROM_CIVIL);
		Self::emit_list(f, &[I::LocalGet(DAYS), I::I64Add, I::LocalSet(EPOCH_DAY)]);
		// a date's position is its day, a local time's the nanoseconds since 1970 as if UTC
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::LocalGet(EPOCH_DAY), I::LocalGet(EPOCH_DAY), I::I64Const(DAY_NANOS), I::I64Mul,
			I::LocalGet(DAY_PART_NANOS), I::I64Add, I::LocalGet(NANOS), I::I64Add,
		]);
		Self::emit_list(f, &is_form_date());
		Self::emit_list(f, &[I::Select, I::LocalSet(POSITION_A)]);
		if self.ctx.required_functions.contains(ZONE_NAMES) {
			self.emit_zoned_finish(f);
		}
		self.emit_return_time(f, KIND_A, POSITION_A);
	}

	/// A zoned time: exact durations move the instant; calendar ones move the wall time, which is then resolved
	/// again (calendar.rs `Time::add`). The wall time carries on as a local time, IS_ZONED remembers to resolve it
	fn emit_zoned_start(&mut self, f: &mut Function) {
		let is_zoned = [I::LocalGet(FORM), I::I64Const(TimeForm::Zoned as i64), I::I64Eq];
		Self::emit_if(f, &is_zoned, |f| {
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(KIND_A), I::I64Const(DAYS_SHIFT), I::I64ShrU, I::I64Const(ZONE_INDEX_MASK), I::I64And,
				I::LocalSet(ZONE),
			]);
			#[rustfmt::skip]
			let has_calendar_parts = vec![
				I::LocalGet(MONTHS), I::I64Const(0), I::I64Ne, I::LocalGet(DAYS), I::I64Const(0), I::I64Ne, I::I32Or,
				I::I32Eqz,
			];
			Self::emit_if(f, &has_calendar_parts, |f| {
				Self::emit_list(f, &[I::LocalGet(ZONE), I::LocalGet(POSITION_A), I::LocalGet(NANOS), I::I64Add]);
				self.call(f, ZONED_NODE);
				f.instruction(&I::Return);
			});
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(KIND_A), I::I64Const(KIND_BITS + OFFSET_SHIFT), I::I64ShrS,
				I::I64Const(MINUTE * SECOND as i64), I::I64Mul, I::LocalGet(POSITION_A), I::I64Add,
				I::LocalSet(POSITION_A),
				I::I64Const(TimeForm::Local as i64), I::LocalSet(FORM),
				I::LocalGet(NANOS), I::LocalSet(EXACT_NANOS),
				I::I64Const(0), I::LocalSet(NANOS),
				I::I64Const(1), I::LocalSet(IS_ZONED),
			]);
		});
	}

	/// The shifted wall time of a zoned time resolved in its zone (no choice: a skipped or repeated one fails), then moved by the exact part
	fn emit_zoned_finish(&mut self, f: &mut Function) {
		Self::emit_if(f, &[I::LocalGet(IS_ZONED), I::I64Const(0), I::I64Ne], |f| {
			Self::emit_list(f, &[I::I64Const(REJECT), I::LocalSet(SPARE)]);
			self.emit_resolved_instant(f, ZONE, POSITION_A, SPARE, SIGN);
			Self::emit_list(f, &[I::LocalGet(ZONE), I::LocalGet(POSITION_A), I::LocalGet(EXACT_NANOS), I::I64Add]);
			self.call(f, ZONED_NODE);
			f.instruction(&I::Return);
		});
	}

	/// `a - b` of two times of one form: days between dates, else an exact duration
	fn emit_time_difference(&mut self, f: &mut Function) {
		let form_of = |kind: u32| {
			vec![I::LocalGet(kind), I::I64Const(KIND_BITS), I::I64ShrU, I::I64Const(TIME_FORM_MASK), I::I64And]
		};
		Self::emit_if(f, &[form_of(KIND_A), form_of(KIND_B), vec![I::I64Ne]].concat(), |f| {
			self.emit_time_failure(f, MISMATCHED)
		});
		Self::emit_list(f, &[I::LocalGet(POSITION_A), I::LocalGet(POSITION_B), I::I64Sub, I::LocalSet(NANOS)]);
		Self::emit_if(
			f,
			&[
				I::LocalGet(KIND_A),
				I::I64Const(KIND_BITS),
				I::I64ShrU,
				I::I64Const(TIME_FORM_MASK),
				I::I64And,
				I::I64Const(TimeForm::Date as i64),
				I::I64Eq,
			],
			|f| {
				f.instruction(&I::LocalGet(NANOS));
				self.call(f, "new_int");
				f.instruction(&I::Return);
			},
		);
		Self::emit_list(f, &[I::I64Const(0), I::LocalSet(MONTHS), I::I64Const(0), I::LocalSet(DAYS)]);
		self.emit_return_duration(f);
	}

	/// The duration of MONTHS, DAYS and NANOS as the result
	fn emit_return_duration(&mut self, f: &mut Function) {
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::I64Const(DURATION_KIND), I::LocalGet(DAYS), I::I64Const(DAYS_MASK), I::I64And, I::I64Const(DAYS_SHIFT),
			I::I64Shl, I::I64Or, I::LocalGet(MONTHS), I::I64Const(MONTHS_IN_KIND), I::I64Shl, I::I64Or,
			I::LocalSet(KIND_A),
		]);
		self.emit_return_time(f, KIND_A, NANOS);
	}

	/// The time node of a kind word and a position, as the result
	fn emit_return_time(&mut self, f: &mut Function, kind: u32, position: u32) {
		let (node, i64_box) = (self.type_manager.node_type, self.type_manager.i64_box_type);
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::LocalGet(kind), I::LocalGet(position), I::StructNew(i64_box), I::RefNull(HeapType::Concrete(node)),
			I::StructNew(node), I::Return,
		]);
	}

	pub(in crate::wasm_emitter) fn emit_time_failure(&mut self, f: &mut Function, message: &str) {
		self.emit_trap_detail(f, &Node::Text(message.to_string()));
		self.emit_runtime_error(f, RETURNED_ERROR);
	}

	fn emit_if(f: &mut Function, condition: &[I<'static>], body: impl FnOnce(&mut Function)) {
		Self::emit_list(f, condition);
		f.instruction(&I::If(BlockType::Empty));
		body(f);
		f.instruction(&I::End);
	}
}

fn is_form_date() -> Emit {
	vec![I::LocalGet(FORM), I::I64Const(TimeForm::Date as i64), I::I64Eq]
}

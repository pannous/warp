//! Times at run time (cards runtime-dates and run-time-dates, notes/dates_at_run_time.md): a $Node of Kind::Time, its
//! TimeForm in the info bits, its data an $i64box of the position (days since 1970 for a date, nanoseconds since 1970
//! for an instant and a local time). `now` is instant_at(clock()), a constant time_of(form, position) (time.rs).
//! time_text shows a time as calendar.rs does, `2026-10-10T14:43:33.682Z`, with Howard Hinnant's civil calendar

use super::layout::BYTE;
use super::list_ops::RETURNED_ERROR;
use super::WasmGcEmitter;
use crate::node::Node;
use crate::time::calendar::{DAYS_PER_ERA, DAY_SECONDS, EPOCH_SHIFT, HOUR, MINUTE, SECOND, YEARS_PER_ERA};
use crate::time::{TimeForm, INSTANT_AT, TIME_FORMS, TIME_FORM_MASK};
use crate::type_kinds::{Kind, KIND_BITS, KIND_MASK};
use wasm_encoder::*;
use Instruction as I;

/// time_field(node, field) -> i64: the field of TIME_FIELDS at that index; the caller has refused the ones the form lacks
pub const TIME_FIELD: &str = "time_field";
/// time_text(node) -> text: a time in RFC 3339, for list_join (print, str)
pub const TIME_TEXT: &str = "time_text";
/// civil_from_days(days) -> (year, month, day)
const CIVIL_FROM_DAYS: &str = "civil_from_days";
/// days_from_civil(year, month, day) -> days
const DAYS_FROM_CIVIL: &str = "days_from_civil";
/// time_parts(form, position) -> (days, nanoseconds of the day): the calendar day and the clock of a position
const TIME_PARTS: &str = "time_parts";
/// The fields time_field reads, as calendar::Time::field names them
const TIME_FIELDS: [&str; 9] = ["year", "month", "day", "weekday", "day_of_year", "hour", "minute", "second", "epoch_seconds"];
const NANOS_PER_MILLI: i64 = 1_000_000;
const DAY_NANOS: i64 = DAY_SECONDS * SECOND as i64;
/// `YYYY-MM-DDThh:mm:ss.fffffffffZ` is 30 bytes
const TIME_TEXT_BYTES: i32 = 32;
const ZERO_DIGIT: i64 = b'0' as i64;
const MISMATCHED_TIMES: &str = "cannot compare a date, a local time and an instant with each other: no implicit conversion between kinds of time";

impl WasmGcEmitter {
	/// After new_int; before list_join, whose time_text calls them
	pub(super) fn emit_times_runtime(&mut self) {
		if !self.should_emit_function(TIME_FIELD) {
			return;
		}
		let node = self.type_manager.node_type;
		let i64_box = self.type_manager.i64_box_type;
		let node_ref = ValType::Ref(self.node_ref(false));
		if self.should_emit_function(INSTANT_AT) {
			self.runtime_function(INSTANT_AT, vec![ValType::I64], vec![node_ref], vec![], |_, f| {
				Self::emit_list(f, &[
					I::I64Const(Kind::Time as i64), I::LocalGet(0), I::I64Const(NANOS_PER_MILLI), I::I64Mul, I::StructNew(i64_box),
					I::RefNull(HeapType::Concrete(node)), I::StructNew(node),
				]);
			});
		}
		self.emit_civil_from_days();
		self.emit_days_from_civil();
		self.runtime_function(TIME_PARTS, vec![ValType::I64; 2], vec![ValType::I64; 2], vec![ValType::I64; 2], |_, f| {
			let (form, position, days, nanos) = (0, 1, 2, 3);
			Self::emit_list(f, &[I::LocalGet(form), I::I64Const(TimeForm::Date as i64), I::I64Eq, I::If(BlockType::Empty), I::LocalGet(position), I::I64Const(0), I::Return, I::End]);
			Self::emit_floor_split(f, position, DAY_NANOS, days, nanos);
			Self::emit_list(f, &[I::LocalGet(days), I::LocalGet(nanos)]);
		});
		self.emit_time_field();
	}

	fn emit_time_field(&mut self) {
		let node_ref = ValType::Ref(self.node_ref(false));
		let local_offset = self.ffi_func_index(crate::host::LOCAL_OFFSET);
		self.runtime_function(TIME_FIELD, vec![node_ref, ValType::I32], vec![ValType::I64], vec![ValType::I64; 8], |s, f| {
			let (field, days, nanos, year, month, day, form, position, milliseconds) = (1, 2, 3, 4, 5, 6, 7, 8, 9);
			s.emit_time_form(f, 0);
			f.instruction(&I::LocalSet(form));
			s.emit_time_position(f, 0);
			f.instruction(&I::LocalSet(position));
			// an instant's wall clock is the environment's: its offset at that instant from the host
			if let Some(local_offset) = local_offset {
				let epoch_seconds = TIME_FIELDS.iter().position(|known| *known == crate::time::EPOCH_SECONDS).unwrap_or_default();
				Self::emit_list(f, &[I::LocalGet(form), I::I64Const(TimeForm::Instant as i64), I::I64Eq]);
				Self::emit_list(f, &[I::LocalGet(field), I::I32Const(epoch_seconds as i32), I::I32Ne, I::I32And, I::If(BlockType::Empty)]);
				Self::emit_floor_split(f, position, NANOS_PER_MILLI, milliseconds, nanos);
				Self::emit_list(f, &[I::LocalGet(position), I::LocalGet(milliseconds), I::Call(local_offset), I::I64Const(SECOND as i64), I::I64Mul, I::I64Add, I::LocalSet(position), I::End]);
			}
			Self::emit_list(f, &[I::LocalGet(form), I::LocalGet(position)]);
			s.call(f, TIME_PARTS);
			Self::emit_list(f, &[I::LocalSet(nanos), I::LocalSet(days), I::LocalGet(days)]);
			s.call(f, CIVIL_FROM_DAYS);
			Self::emit_list(f, &[I::LocalSet(day), I::LocalSet(month), I::LocalSet(year)]);
			let mut day_of_year = vec![I::LocalGet(days), I::LocalGet(year), I::I64Const(1), I::I64Const(1), I::Call(s.func_index(DAYS_FROM_CIVIL))];
			day_of_year.extend([I::I64Sub, I::I64Const(1), I::I64Add]);
			let values: [Vec<I<'static>>; 9] = [
				vec![I::LocalGet(year)],
				vec![I::LocalGet(month)],
				vec![I::LocalGet(day)],
				// ISO weekday: 1970-01-01 was a Thursday, Monday is 1
				vec![I::LocalGet(days), I::I64Const(3), I::I64Add, I::I64Const(7), I::I64RemS, I::I64Const(7), I::I64Add, I::I64Const(7), I::I64RemS, I::I64Const(1), I::I64Add],
				day_of_year,
				vec![I::LocalGet(nanos), I::I64Const(HOUR * SECOND as i64), I::I64DivS],
				vec![I::LocalGet(nanos), I::I64Const(MINUTE * SECOND as i64), I::I64DivS, I::I64Const(MINUTE), I::I64RemS],
				vec![I::LocalGet(nanos), I::I64Const(SECOND as i64), I::I64DivS, I::I64Const(MINUTE), I::I64RemS],
				vec![I::LocalGet(days), I::I64Const(DAY_SECONDS), I::I64Mul, I::LocalGet(nanos), I::I64Const(SECOND as i64), I::I64DivS, I::I64Add],
			];
			for (index, value) in values.iter().enumerate() {
				Self::emit_list(f, &[I::LocalGet(field), I::I32Const(index as i32), I::I32Eq, I::If(BlockType::Empty)]);
				Self::emit_list(f, value);
				Self::emit_list(f, &[I::Return, I::End]);
			}
			f.instruction(&I::Unreachable);
		});
	}

	/// Howard Hinnant's civil_from_days, as calendar::civil_from_days
	fn emit_civil_from_days(&mut self) {
		self.runtime_function(CIVIL_FROM_DAYS, vec![ValType::I64], vec![ValType::I64; 3], vec![ValType::I64; 7], |_, f| {
			let (days, era, day_of_era, year_of_era, day_of_year, shifted_month, month) = (0, 1, 2, 3, 4, 5, 6);
			Self::emit_list(f, &[I::LocalGet(days), I::I64Const(EPOCH_SHIFT), I::I64Add, I::LocalSet(days)]);
			Self::emit_floor_split(f, days, DAYS_PER_ERA, era, day_of_era);
			// (doe - doe/1460 + doe/36524 - doe/146096) / 365
			Self::emit_list(f, &[
				I::LocalGet(day_of_era), I::LocalGet(day_of_era), I::I64Const(1460), I::I64DivU, I::I64Sub,
				I::LocalGet(day_of_era), I::I64Const(36_524), I::I64DivU, I::I64Add,
				I::LocalGet(day_of_era), I::I64Const(146_096), I::I64DivU, I::I64Sub, I::I64Const(365), I::I64DivU, I::LocalSet(year_of_era),
			]);
			// doe - (365 yoe + yoe/4 - yoe/100)
			Self::emit_list(f, &[
				I::LocalGet(day_of_era), I::LocalGet(year_of_era), I::I64Const(365), I::I64Mul,
				I::LocalGet(year_of_era), I::I64Const(4), I::I64DivU, I::I64Add,
				I::LocalGet(year_of_era), I::I64Const(100), I::I64DivU, I::I64Sub, I::I64Sub, I::LocalSet(day_of_year),
			]);
			Self::emit_list(f, &[I::LocalGet(day_of_year), I::I64Const(5), I::I64Mul, I::I64Const(2), I::I64Add, I::I64Const(153), I::I64DivU, I::LocalSet(shifted_month)]);
			// the year starts in March: months 10 and 11 of it are January and February
			Self::emit_list(f, &[
				I::LocalGet(shifted_month), I::I64Const(3), I::I64Add, I::LocalGet(shifted_month), I::I64Const(10), I::I64GeU, I::I64ExtendI32U,
				I::I64Const(12), I::I64Mul, I::I64Sub, I::LocalSet(month),
			]);
			// year, month, day
			Self::emit_list(f, &[
				I::LocalGet(year_of_era), I::LocalGet(era), I::I64Const(YEARS_PER_ERA), I::I64Mul, I::I64Add,
				I::LocalGet(month), I::I64Const(2), I::I64LeU, I::I64ExtendI32U, I::I64Add,
				I::LocalGet(month),
				I::LocalGet(day_of_year), I::LocalGet(shifted_month), I::I64Const(153), I::I64Mul, I::I64Const(2), I::I64Add,
				I::I64Const(5), I::I64DivU, I::I64Sub, I::I64Const(1), I::I64Add,
			]);
		});
	}

	/// Howard Hinnant's days_from_civil, as calendar::days_from_civil
	fn emit_days_from_civil(&mut self) {
		self.runtime_function(DAYS_FROM_CIVIL, vec![ValType::I64; 3], vec![ValType::I64], vec![ValType::I64; 4], |_, f| {
			let (year, month, day, era, year_of_era, day_of_year, shifted_month) = (0, 1, 2, 3, 4, 5, 6);
			// the year starts in March
			Self::emit_list(f, &[I::LocalGet(year), I::LocalGet(month), I::I64Const(2), I::I64LeS, I::I64ExtendI32U, I::I64Sub, I::LocalSet(year)]);
			Self::emit_floor_split(f, year, YEARS_PER_ERA, era, year_of_era);
			Self::emit_list(f, &[
				I::LocalGet(month), I::I64Const(9), I::I64Add, I::I64Const(12), I::I64RemS, I::LocalSet(shifted_month),
				I::LocalGet(shifted_month), I::I64Const(153), I::I64Mul, I::I64Const(2), I::I64Add, I::I64Const(5), I::I64DivS,
				I::LocalGet(day), I::I64Add, I::I64Const(1), I::I64Sub, I::LocalSet(day_of_year),
				I::LocalGet(era), I::I64Const(DAYS_PER_ERA), I::I64Mul,
				I::LocalGet(year_of_era), I::I64Const(365), I::I64Mul, I::LocalGet(year_of_era), I::I64Const(4), I::I64DivS, I::I64Add,
				I::LocalGet(year_of_era), I::I64Const(100), I::I64DivS, I::I64Sub, I::LocalGet(day_of_year), I::I64Add,
				I::I64Add, I::I64Const(EPOCH_SHIFT), I::I64Sub,
			]);
		});
	}

	/// After int_to_decimal and the times runtime, which it calls
	pub(super) fn emit_time_text(&mut self) {
		if !self.should_emit_function(TIME_TEXT) {
			return;
		}
		self.emit_text_heap_global();
		let node_ref = ValType::Ref(self.node_ref(false));
		let mut locals = vec![ValType::I64; 8];
		locals.extend([ValType::I32; 2]);
		self.runtime_function(TIME_TEXT, vec![node_ref], vec![node_ref], locals, |s, f| {
			let (form, days, nanos, year, month, day, fraction, power, position, address) = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
			let advance = |f: &mut Function, count: i32| Self::emit_list(f, &[I::LocalGet(position), I::I32Const(count), I::I32Add, I::LocalSet(position)]);
			let put = |f: &mut Function, byte: u8| {
				Self::emit_list(f, &[I::LocalGet(position), I::I32Const(byte as i32), I::I32Store8(BYTE)]);
				advance(f, 1);
			};
			// the value `count` digits wide, with leading zeros
			let digits = |f: &mut Function, value: &[I<'static>], count: u32| {
				for place in 0..count {
					Self::emit_list(f, &[I::LocalGet(position), I::I32Const(place as i32), I::I32Add]);
					Self::emit_list(f, value);
					Self::emit_list(f, &[I::I64Const(10_i64.pow(count - 1 - place)), I::I64DivU, I::I64Const(10), I::I64RemU]);
					Self::emit_list(f, &[I::I64Const(ZERO_DIGIT), I::I64Add, I::I32WrapI64, I::I32Store8(BYTE)]);
				}
				advance(f, count as i32);
			};
			let is_form = |f: &mut Function, time_form: TimeForm| Self::emit_list(f, &[I::LocalGet(form), I::I64Const(time_form as i64), I::I64Eq]);

			s.emit_time_form(f, 0);
			f.instruction(&I::LocalSet(form));
			f.instruction(&I::LocalGet(form));
			s.emit_time_position(f, 0);
			s.call(f, TIME_PARTS);
			Self::emit_list(f, &[I::LocalSet(nanos), I::LocalSet(days), I::LocalGet(days)]);
			s.call(f, CIVIL_FROM_DAYS);
			Self::emit_list(f, &[I::LocalSet(day), I::LocalSet(month), I::LocalSet(year)]);
			Self::emit_list(f, &[I::I32Const(TIME_TEXT_BYTES), I::LocalSet(position)]);
			s.emit_text_allocation(f, position, address);
			Self::emit_list(f, &[I::LocalGet(address), I::LocalSet(position)]);
			digits(f, &[I::LocalGet(year)], 4);
			put(f, b'-');
			digits(f, &[I::LocalGet(month)], 2);
			put(f, b'-');
			digits(f, &[I::LocalGet(day)], 2);
			// a date has no clock
			is_form(f, TimeForm::Date);
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty)]);
			put(f, b'T');
			digits(f, &[I::LocalGet(nanos), I::I64Const(HOUR * SECOND as i64), I::I64DivU], 2);
			put(f, b':');
			digits(f, &[I::LocalGet(nanos), I::I64Const(MINUTE * SECOND as i64), I::I64DivU, I::I64Const(MINUTE), I::I64RemU], 2);
			// seconds only when there are any, the fraction without its trailing zeros
			Self::emit_list(f, &[I::LocalGet(nanos), I::I64Const(SECOND as i64), I::I64RemU, I::LocalSet(fraction)]);
			let whole_seconds = [I::LocalGet(nanos), I::I64Const(SECOND as i64), I::I64DivU, I::I64Const(MINUTE), I::I64RemU];
			Self::emit_list(f, &whole_seconds);
			Self::emit_list(f, &[I::LocalGet(fraction), I::I64Or, I::I64Const(0), I::I64Ne, I::If(BlockType::Empty)]);
			put(f, b':');
			digits(f, &whole_seconds, 2);
			f.instruction(&I::End);
			Self::emit_list(f, &[I::LocalGet(fraction), I::I64Const(0), I::I64Ne, I::If(BlockType::Empty)]);
			Self::emit_list(f, &[I::I64Const(SECOND as i64), I::LocalSet(power)]);
			Self::emit_while(f, &[I::LocalGet(fraction), I::I64Const(10), I::I64RemU, I::I64Eqz], &[
				I::LocalGet(fraction), I::I64Const(10), I::I64DivU, I::LocalSet(fraction),
				I::LocalGet(power), I::I64Const(10), I::I64DivU, I::LocalSet(power),
			]);
			// power + fraction keeps the fraction's leading zeros; the point overwrites its leading 1
			Self::emit_list(f, &[I::LocalGet(position), I::LocalGet(power), I::LocalGet(fraction), I::I64Add]);
			s.call(f, "int_to_decimal");
			Self::emit_list(f, &[I::LocalGet(position), I::I32Const(b'.' as i32), I::I32Store8(BYTE)]);
			Self::emit_list(f, &[I::LocalGet(position), I::I32Add, I::LocalSet(position), I::End]);
			is_form(f, TimeForm::Instant);
			f.instruction(&I::If(BlockType::Empty));
			put(f, b'Z');
			Self::emit_list(f, &[I::End, I::End]);
			Self::emit_list(f, &[I::LocalGet(address), I::LocalGet(position), I::LocalGet(address), I::I32Sub]);
			s.call(f, "new_text");
		});
	}

	/// time_of(form, position) of two constants: the node itself
	pub(super) fn emit_time_constant(&self, func: &mut Function, form: i64, position: i64) {
		let (node, i64_box) = (self.type_manager.node_type, self.type_manager.i64_box_type);
		Self::emit_list(func, &[
			I::I64Const((form << KIND_BITS) | Kind::Time as i64), I::I64Const(position), I::StructNew(i64_box),
			I::RefNull(HeapType::Concrete(node)), I::StructNew(node),
		]);
	}

	/// Two times in node_order, whose locals `kinds` hold their kinds: the earlier one first; only the same form compares
	pub(super) fn emit_time_order(&mut self, func: &mut Function, kinds: [u32; 2], first: u32, second: u32) {
		if !self.should_emit_function(TIME_FIELD) {
			return;
		}
		for kind in kinds {
			Self::emit_list(func, &[I::LocalGet(kind), I::I64Const(Kind::Time as i64), I::I64Eq]);
		}
		Self::emit_list(func, &[I::I32And, I::If(BlockType::Empty)]);
		self.emit_time_form(func, first);
		self.emit_time_form(func, second);
		Self::emit_list(func, &[I::I64Ne, I::If(BlockType::Empty)]);
		self.emit_trap_detail(func, &Node::Text(MISMATCHED_TIMES.to_string()));
		self.emit_runtime_error(func, RETURNED_ERROR);
		func.instruction(&I::End);
		for comparison in [I::I64GtS, I::I64LtS] {
			self.emit_time_position(func, first);
			self.emit_time_position(func, second);
			func.instruction(&comparison);
		}
		Self::emit_list(func, &[I::I32Sub, I::Return, I::End]);
	}

	/// `t.month` of a time: its field, or the refusal of a form without it (`instant has no hour`, `date has no hour`)
	pub(super) fn emit_time_field_lookup(&mut self, func: &mut Function, held: u32, name: &str) {
		if !self.should_emit_function(TIME_FIELD) {
			return;
		}
		let Some(field) = TIME_FIELDS.iter().position(|known| *known == name) else { return };
		self.emit_field(func, held, 0);
		Self::emit_list(func, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Time as i64), I::I64Eq, I::If(BlockType::Empty)]);
		for form in TIME_FORMS {
			if let Err(refusal) = form.example().field(name) {
				self.emit_time_form(func, held);
				Self::emit_list(func, &[I::I64Const(form as i64), I::I64Eq, I::If(BlockType::Empty)]);
				self.emit_trap_detail(func, &Node::Text(refusal));
				self.emit_runtime_error(func, RETURNED_ERROR);
				func.instruction(&I::End);
			}
		}
		Self::emit_list(func, &[I::LocalGet(held), I::RefAsNonNull, I::I32Const(field as i32)]);
		self.call(func, TIME_FIELD);
		self.call(func, "new_int");
		Self::emit_list(func, &[I::Br(1), I::End]);
	}

	/// The TimeForm of the time node in `local`
	fn emit_time_form(&self, func: &mut Function, local: u32) {
		self.emit_field(func, local, 0);
		Self::emit_list(func, &[I::I64Const(KIND_BITS), I::I64ShrU, I::I64Const(TIME_FORM_MASK), I::I64And]);
	}

	/// The position of the time node in `local`
	fn emit_time_position(&self, func: &mut Function, local: u32) {
		let i64_box = self.type_manager.i64_box_type;
		self.emit_field(func, local, 1);
		Self::emit_list(func, &[I::RefCastNonNull(HeapType::Concrete(i64_box)), I::StructGet { struct_type_index: i64_box, field_index: 0 }]);
	}

	/// Quotient and remainder rounded toward minus infinity, as div_euclid and rem_euclid
	fn emit_floor_split(func: &mut Function, dividend: u32, divisor: i64, quotient: u32, remainder: u32) {
		Self::emit_list(func, &[I::LocalGet(dividend), I::I64Const(divisor), I::I64DivS, I::LocalSet(quotient)]);
		Self::emit_list(func, &[I::LocalGet(dividend), I::LocalGet(quotient), I::I64Const(divisor), I::I64Mul, I::I64Sub, I::LocalTee(remainder)]);
		Self::emit_list(func, &[I::I64Const(0), I::I64LtS, I::If(BlockType::Empty)]);
		Self::emit_list(func, &[I::LocalGet(remainder), I::I64Const(divisor), I::I64Add, I::LocalSet(remainder)]);
		Self::emit_list(func, &[I::LocalGet(quotient), I::I64Const(1), I::I64Sub, I::LocalSet(quotient), I::End]);
	}
}

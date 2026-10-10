//! Instants at run time (card runtime-dates, notes/dates_at_run_time.md): `now` is instant_at(clock()), a $Node of
//! Kind::Time whose data is an $i64box of nanoseconds since 1970 UTC; instant_text shows it as calendar.rs does,
//! `2026-10-10T14:43:33.682Z`, with Howard Hinnant's civil calendar arithmetic

use super::layout::BYTE;
use super::list_ops::RETURNED_ERROR;
use super::WasmGcEmitter;
use crate::node::Node;
use crate::time::calendar::{no_wall_clock, DAYS_PER_ERA, DAY_SECONDS, EPOCH_SHIFT, HOUR, MINUTE, SECOND, YEARS_PER_ERA};
use crate::time::INSTANT_AT;
use crate::type_kinds::{Kind, KIND_MASK};
use wasm_encoder::*;
use Instruction as I;

/// instant_text(node) -> text: an instant in RFC 3339, for list_join (print, str)
pub const INSTANT_TEXT: &str = "instant_text";
const NANOS_PER_MILLI: i64 = 1_000_000;
/// `YYYY-MM-DDThh:mm:ss.fffffffffZ` is 30 bytes
const INSTANT_TEXT_BYTES: i32 = 32;
const ZERO_DIGIT: i64 = b'0' as i64;

impl WasmGcEmitter {
	pub(super) fn emit_times_runtime(&mut self) {
		if !self.should_emit_function(INSTANT_AT) {
			return;
		}
		let node = self.type_manager.node_type;
		let i64_box = self.type_manager.i64_box_type;
		let node_ref = ValType::Ref(self.node_ref(false));
		self.runtime_function(INSTANT_AT, vec![ValType::I64], vec![node_ref], vec![], |_, f| {
			Self::emit_list(f, &[
				I::I64Const(Kind::Time as i64), I::LocalGet(0), I::I64Const(NANOS_PER_MILLI), I::I64Mul, I::StructNew(i64_box),
				I::RefNull(HeapType::Concrete(node)), I::StructNew(node),
			]);
		});
	}

	/// After int_to_decimal, which it calls
	pub(super) fn emit_instant_text(&mut self) {
		if !self.should_emit_function(INSTANT_TEXT) {
			return;
		}
		self.emit_text_heap_global();
		let i64_box = self.type_manager.i64_box_type;
		let node_ref = ValType::Ref(self.node_ref(false));
		let mut locals = vec![ValType::I64; 14];
		locals.extend([ValType::I32; 2]);
		self.runtime_function(INSTANT_TEXT, vec![node_ref], vec![node_ref], locals, |s, f| {
			let (nanos, seconds, days, era, day_of_era, year_of_era, day_of_year, shifted_month) = (1, 2, 3, 4, 5, 6, 7, 8);
			let (month, year, day, second_of_day, fraction, power, position, address) = (9, 10, 11, 12, 13, 14, 15, 16);
			// quotient and remainder rounded toward minus infinity, as div_euclid and rem_euclid
			let floor_split = |f: &mut Function, dividend: u32, divisor: i64, quotient: u32, remainder: u32| {
				Self::emit_list(f, &[I::LocalGet(dividend), I::I64Const(divisor), I::I64DivS, I::LocalSet(quotient)]);
				Self::emit_list(f, &[I::LocalGet(dividend), I::LocalGet(quotient), I::I64Const(divisor), I::I64Mul, I::I64Sub, I::LocalTee(remainder)]);
				Self::emit_list(f, &[I::I64Const(0), I::I64LtS, I::If(BlockType::Empty)]);
				Self::emit_list(f, &[I::LocalGet(remainder), I::I64Const(divisor), I::I64Add, I::LocalSet(remainder)]);
				Self::emit_list(f, &[I::LocalGet(quotient), I::I64Const(1), I::I64Sub, I::LocalSet(quotient), I::End]);
			};
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

			s.emit_field(f, 0, 1);
			Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(i64_box)), I::StructGet { struct_type_index: i64_box, field_index: 0 }, I::LocalSet(nanos)]);
			floor_split(f, nanos, SECOND as i64, seconds, fraction);
			floor_split(f, seconds, DAY_SECONDS, days, second_of_day);
			Self::emit_list(f, &[I::LocalGet(days), I::I64Const(EPOCH_SHIFT), I::I64Add, I::LocalSet(days)]);
			floor_split(f, days, DAYS_PER_ERA, era, day_of_era);
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
			Self::emit_list(f, &[
				I::LocalGet(day_of_year), I::LocalGet(shifted_month), I::I64Const(153), I::I64Mul, I::I64Const(2), I::I64Add,
				I::I64Const(5), I::I64DivU, I::I64Sub, I::I64Const(1), I::I64Add, I::LocalSet(day),
			]);
			// the year starts in March: months 10 and 11 of it are January and February
			Self::emit_list(f, &[
				I::LocalGet(shifted_month), I::I64Const(3), I::I64Add, I::LocalGet(shifted_month), I::I64Const(10), I::I64GeU, I::I64ExtendI32U,
				I::I64Const(12), I::I64Mul, I::I64Sub, I::LocalSet(month),
			]);
			Self::emit_list(f, &[
				I::LocalGet(year_of_era), I::LocalGet(era), I::I64Const(YEARS_PER_ERA), I::I64Mul, I::I64Add,
				I::LocalGet(month), I::I64Const(2), I::I64LeU, I::I64ExtendI32U, I::I64Add, I::LocalSet(year),
			]);

			Self::emit_list(f, &[I::I32Const(INSTANT_TEXT_BYTES), I::LocalSet(position)]);
			s.emit_text_allocation(f, position, address);
			Self::emit_list(f, &[I::LocalGet(address), I::LocalSet(position)]);
			digits(f, &[I::LocalGet(year)], 4);
			put(f, b'-');
			digits(f, &[I::LocalGet(month)], 2);
			put(f, b'-');
			digits(f, &[I::LocalGet(day)], 2);
			put(f, b'T');
			digits(f, &[I::LocalGet(second_of_day), I::I64Const(HOUR), I::I64DivU], 2);
			put(f, b':');
			digits(f, &[I::LocalGet(second_of_day), I::I64Const(MINUTE), I::I64DivU, I::I64Const(MINUTE), I::I64RemU], 2);
			// seconds only when there are any, the fraction without its trailing zeros
			let whole_seconds = [I::LocalGet(second_of_day), I::I64Const(MINUTE), I::I64RemU];
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
			put(f, b'Z');
			Self::emit_list(f, &[I::LocalGet(address), I::LocalGet(position), I::LocalGet(address), I::I32Sub]);
			s.call(f, "new_text");
		});
	}

	/// Two instants in node_order, whose locals `kinds` hold their kinds: the earlier one first
	pub(super) fn emit_instant_order(&self, func: &mut Function, kinds: [u32; 2], first: u32, second: u32) {
		if !self.should_emit_function(INSTANT_AT) {
			return;
		}
		for kind in kinds {
			Self::emit_list(func, &[I::LocalGet(kind), I::I64Const(Kind::Time as i64), I::I64Eq]);
		}
		Self::emit_list(func, &[I::I32And, I::If(BlockType::Empty)]);
		let i64_box = self.type_manager.i64_box_type;
		let nanos = |func: &mut Function, node: u32| {
			self.emit_field(func, node, 1);
			Self::emit_list(func, &[I::RefCastNonNull(HeapType::Concrete(i64_box)), I::StructGet { struct_type_index: i64_box, field_index: 0 }]);
		};
		for comparison in [I::I64GtS, I::I64LtS] {
			nanos(func, first);
			nanos(func, second);
			func.instruction(&comparison);
		}
		Self::emit_list(func, &[I::I32Sub, I::Return, I::End]);
	}

	/// `t.hour` of an instant fails: it has a wall clock only in a zone
	pub(super) fn emit_instant_field_lookup(&mut self, func: &mut Function, held: u32, name: &str) {
		if !self.should_emit_function(INSTANT_AT) {
			return;
		}
		self.emit_field(func, held, 0);
		Self::emit_list(func, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Time as i64), I::I64Eq, I::If(BlockType::Empty)]);
		self.emit_trap_detail(func, &Node::Text(no_wall_clock(name)));
		self.emit_runtime_error(func, RETURNED_ERROR);
		func.instruction(&I::End);
	}
}

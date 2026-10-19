//! Times at run time (cards runtime-dates and run-time-dates, notes/dates_at_run_time.md): a $Node of Kind::Time, its
//! TimeForm in the info bits, its data an $i64box of the position (days since 1970 for a date, nanoseconds since 1970
//! for an instant and a local time). `now` is instant_at(clock()), a constant time_of(form, position) (time.rs).
//! time_text shows a time as calendar.rs does, `2026-10-10T14:43:33.682Z`, with Howard Hinnant's civil calendar

use super::super::lists::list_ops::RETURNED_ERROR;
use super::super::time::time_arithmetic::DURATION_ORDER;
use super::super::WasmGcEmitter;
use crate::node::Node;
use crate::time::calendar::{
	BUILTIN_RULES, DATE_FIELD, DAYS_PER_ERA, DAY_SECONDS, EPOCH_SHIFT, HOUR, MINUTE, SECOND, TIME_OF_DAY_FIELD,
	YEARS_PER_ERA, ZONES,
};
use crate::time::{
	TimeForm, INSTANT_AT, MONTHS_SHIFT, OFFSET_SHIFT, TIME_FORMS, TIME_FORM_BITS, TIME_FORM_MASK, TZDATA_FIELD,
	ZONE_INDEX_MASK,
};
use crate::type_kinds::{Kind, KIND_BITS};
use wasm_encoder::*;
use Instruction as I;

/// time_field(node, field) -> i64: the field of TIME_FIELDS at that index; the caller has refused the ones the form lacks
pub const TIME_FIELD: &str = "time_field";
/// lib/prelude.warp's time_field_of, by its prelude name: the calendar of time_field in warp (card time-fields-warp)
const TIME_FIELD_WORD: &str = "prelude·time_field_of";
const TIME_FIELD_NAME: &str = "time_field_of";
/// time_text(node) -> text: a time in RFC 3339, for list_join (print, str)
pub const TIME_TEXT: &str = "time_text";
/// duration_text(node) -> text: a duration as calendar.rs shows it, `1 month 2 days 3 hours`
const DURATION_TEXT: &str = "duration_text";
/// Required by a zoned constant: time_text then names the zones (`[Europe/Berlin]`)
pub const ZONE_NAMES: &str = "zone_names";
/// civil_from_days(days) -> (year, month, day)
pub(in crate::wasm_emitter) const CIVIL_FROM_DAYS: &str = "civil_from_days";
/// days_from_civil(year, month, day) -> days
pub(in crate::wasm_emitter) const DAYS_FROM_CIVIL: &str = "days_from_civil";
/// time_parts(form, position) -> (days, nanoseconds of the day): the calendar day and the clock of a position
pub(in crate::wasm_emitter) const TIME_PARTS: &str = "time_parts";
/// The fields time_field reads, as calendar::Time::field names them, then the parts (calendar::Time::part) as positions
const TIME_FIELDS: [&str; 12] = [
	"year",
	"month",
	"day",
	"weekday",
	"day_of_year",
	"hour",
	"minute",
	"second",
	"epoch_seconds",
	"offset",
	TIME_OF_DAY_FIELD,
	DATE_FIELD,
];
/// The parts, a time of their form: `now.time` is a time of day, `now.date` a date
const TIME_PARTS_FORMS: [(&str, TimeForm); 2] = [(TIME_OF_DAY_FIELD, TimeForm::Clock), (DATE_FIELD, TimeForm::Date)];
/// The fields that read the position itself, not its wall clock
const ABSOLUTE_FIELDS: [&str; 2] = [crate::time::EPOCH_SECONDS, "offset"];
const NANOS_PER_MILLI: i64 = 1_000_000;
pub(in crate::wasm_emitter) const DAY_NANOS: i64 = DAY_SECONDS * SECOND as i64;
/// `YYYY-MM-DDThh:mm:ss.fffffffff+hh:mm[America/Los_Angeles][_tzdata=warp-2026a]` is 77 bytes
const TIME_TEXT_BYTES: i32 = 80;
/// `-123 months -45 days -6 hours -7 minutes -8.123456789 seconds`, every amount at most 20 bytes
const DURATION_TEXT_BYTES: i32 = 128;
const ZERO_DIGIT: i64 = b'0' as i64;
const MISMATCHED_TIMES: &str = "cannot compare a date, a local time, an instant and a zoned time with each other: no implicit conversion between kinds of time";
/// The offset and length of a constant in the literal segment
type Literal = (u32, u32);

/// The prelude word a time's fields need, for modules IMPLIED_WORDS: `now`, a time call or a date or time literal makes
/// a time, whose fields (and text) time_field reads through time_field_of
pub(crate) fn implied_word(node: &Node) -> Option<&'static str> {
	let makes_time = node.is_symbol(crate::time::NOW_WORD)
		|| crate::time::is_time_call(node)
		|| crate::time::time_data(node.drop_meta());
	makes_time.then_some(TIME_FIELD_NAME)
}

impl WasmGcEmitter {
	/// After new_int; before list_join, whose time_text calls them
	pub(in crate::wasm_emitter) fn emit_times_runtime(&mut self) {
		if !self.should_emit_function(TIME_FIELD) {
			return;
		}
		let node = self.type_manager.node_type;
		let i64_box = self.type_manager.i64_box_type;
		let node_ref = ValType::Ref(self.node_ref(false));
		if self.should_emit_function(INSTANT_AT) {
			self.runtime_function(INSTANT_AT, vec![ValType::I64], vec![node_ref], vec![], |_, f| {
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::I64Const(Kind::Time as i64), I::LocalGet(0), I::I64Const(NANOS_PER_MILLI), I::I64Mul,
					I::StructNew(i64_box), I::RefNull(HeapType::Concrete(node)), I::StructNew(node),
				]);
			});
		}
		self.emit_civil_from_days();
		self.emit_days_from_civil();
		self.runtime_function(
			TIME_PARTS,
			vec![ValType::I64; 2],
			vec![ValType::I64; 2],
			vec![ValType::I64; 2],
			|_, f| {
				let (form, position, days, nanos) = (0, 1, 2, 3);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(form), I::I64Const(TimeForm::Date as i64), I::I64Eq, I::If(BlockType::Empty),
						I::LocalGet(position), I::I64Const(0), I::Return,
					I::End,
				]);
				Self::emit_floor_split(f, position, DAY_NANOS, days, nanos);
				Self::emit_list(f, &[I::LocalGet(days), I::LocalGet(nanos)]);
			},
		);
		self.emit_time_field();
	}

	/// The wall clock position of the time, then its field by lib/prelude.warp time_field_of, called late
	fn emit_time_field(&mut self) {
		let node_ref = ValType::Ref(self.node_ref(false));
		let local_offset = self.ffi_func_index(crate::host::LOCAL_OFFSET);
		let late = self.late_call(TIME_FIELD_WORD, vec![ValType::I64; 4], vec![ValType::I64]);
		self.runtime_function(
			TIME_FIELD,
			vec![node_ref, ValType::I32],
			vec![ValType::I64],
			vec![ValType::I64; 5],
			|s, f| {
				let (field, form, position, milliseconds, nanos, offset) = (1, 2, 3, 4, 5, 6);
				s.emit_time_form(f, 0);
				f.instruction(&I::LocalSet(form));
				s.emit_time_position(f, 0);
				f.instruction(&I::LocalSet(position));
				s.emit_zone_offset(f, 0);
				f.instruction(&I::LocalSet(offset));
				// the wall clock: a zoned time's own offset, an instant's the environment's at that instant, from the host
				f.instruction(&I::I32Const(1));
				for absolute in ABSOLUTE_FIELDS {
					let index = TIME_FIELDS.iter().position(|known| *known == absolute).unwrap_or_default();
					Self::emit_list(f, &[I::LocalGet(field), I::I32Const(index as i32), I::I32Ne, I::I32And]);
				}
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::If(BlockType::Empty),
						I::LocalGet(form), I::I64Const(TimeForm::Zoned as i64), I::I64Eq, I::If(BlockType::Empty),
							I::LocalGet(position), I::LocalGet(offset), I::I64Const(SECOND as i64), I::I64Mul,
							I::I64Add, I::LocalSet(position),
						I::End,
				]);
				if let Some(local_offset) = local_offset {
					Self::emit_list(
						f,
						&[I::LocalGet(form), I::I64Const(TimeForm::Instant as i64), I::I64Eq, I::If(BlockType::Empty)],
					);
					Self::emit_floor_split(f, position, NANOS_PER_MILLI, milliseconds, nanos);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(position), I::LocalGet(milliseconds), I::Call(local_offset),
						I::I64Const(SECOND as i64), I::I64Mul, I::I64Add, I::LocalSet(position),
						I::End,
					]);
				}
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::End,
					I::LocalGet(form), I::LocalGet(position), I::LocalGet(offset), I::LocalGet(field), I::I64ExtendI32U,
				]);
				Self::emit_late_call(f, &late);
			},
		);
	}

	/// Howard Hinnant's civil_from_days, as calendar::civil_from_days
	fn emit_civil_from_days(&mut self) {
		self.runtime_function(
			CIVIL_FROM_DAYS,
			vec![ValType::I64],
			vec![ValType::I64; 3],
			vec![ValType::I64; 7],
			|_, f| {
				let (days, era, day_of_era, year_of_era, day_of_year, shifted_month, month) = (0, 1, 2, 3, 4, 5, 6);
				Self::emit_list(f, &[I::LocalGet(days), I::I64Const(EPOCH_SHIFT), I::I64Add, I::LocalSet(days)]);
				Self::emit_floor_split(f, days, DAYS_PER_ERA, era, day_of_era);
				// (doe - doe/1460 + doe/36524 - doe/146096) / 365
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(day_of_era), I::LocalGet(day_of_era), I::I64Const(1460), I::I64DivU, I::I64Sub,
					I::LocalGet(day_of_era), I::I64Const(36_524), I::I64DivU, I::I64Add, I::LocalGet(day_of_era),
					I::I64Const(146_096), I::I64DivU, I::I64Sub, I::I64Const(365), I::I64DivU, I::LocalSet(year_of_era),
				]);
				// doe - (365 yoe + yoe/4 - yoe/100)
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(day_of_era), I::LocalGet(year_of_era), I::I64Const(365), I::I64Mul,
					I::LocalGet(year_of_era), I::I64Const(4), I::I64DivU, I::I64Add, I::LocalGet(year_of_era),
					I::I64Const(100), I::I64DivU, I::I64Sub, I::I64Sub, I::LocalSet(day_of_year),
					I::LocalGet(day_of_year), I::I64Const(5), I::I64Mul, I::I64Const(2), I::I64Add, I::I64Const(153),
					I::I64DivU, I::LocalSet(shifted_month),
				]);
				// the year starts in March: months 10 and 11 of it are January and February
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(shifted_month), I::I64Const(3), I::I64Add, I::LocalGet(shifted_month), I::I64Const(10),
					I::I64GeU, I::I64ExtendI32U, I::I64Const(12), I::I64Mul, I::I64Sub, I::LocalSet(month),
				]);
				// year, month, day
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(year_of_era), I::LocalGet(era), I::I64Const(YEARS_PER_ERA), I::I64Mul, I::I64Add,
					I::LocalGet(month), I::I64Const(2), I::I64LeU, I::I64ExtendI32U, I::I64Add, I::LocalGet(month),
					I::LocalGet(day_of_year), I::LocalGet(shifted_month), I::I64Const(153), I::I64Mul, I::I64Const(2),
					I::I64Add, I::I64Const(5), I::I64DivU, I::I64Sub, I::I64Const(1), I::I64Add,
				]);
			},
		);
	}

	/// Howard Hinnant's days_from_civil, as calendar::days_from_civil
	fn emit_days_from_civil(&mut self) {
		self.runtime_function(
			DAYS_FROM_CIVIL,
			vec![ValType::I64; 3],
			vec![ValType::I64],
			vec![ValType::I64; 4],
			|_, f| {
				let (year, month, day, era, year_of_era, day_of_year, shifted_month) = (0, 1, 2, 3, 4, 5, 6);
				// the year starts in March
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(year), I::LocalGet(month), I::I64Const(2), I::I64LeS, I::I64ExtendI32U, I::I64Sub,
					I::LocalSet(year),
				]);
				Self::emit_floor_split(f, year, YEARS_PER_ERA, era, year_of_era);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(month), I::I64Const(9), I::I64Add, I::I64Const(12), I::I64RemS,
					I::LocalSet(shifted_month),
					I::LocalGet(shifted_month), I::I64Const(153), I::I64Mul, I::I64Const(2), I::I64Add, I::I64Const(5),
					I::I64DivS, I::LocalGet(day), I::I64Add, I::I64Const(1), I::I64Sub, I::LocalSet(day_of_year),
					I::LocalGet(era), I::I64Const(DAYS_PER_ERA), I::I64Mul, I::LocalGet(year_of_era), I::I64Const(365),
					I::I64Mul, I::LocalGet(year_of_era), I::I64Const(4), I::I64DivS, I::I64Add,
					I::LocalGet(year_of_era), I::I64Const(100), I::I64DivS, I::I64Sub, I::LocalGet(day_of_year),
					I::I64Add, I::I64Add, I::I64Const(EPOCH_SHIFT), I::I64Sub,
				]);
			},
		);
	}

	/// After int_to_decimal and the times runtime, which it calls
	pub(in crate::wasm_emitter) fn emit_time_text(&mut self) {
		if !self.should_emit_function(TIME_TEXT) {
			return;
		}
		self.emit_duration_text();
		let zone_names = self.zone_names();
		let node_ref = ValType::Ref(self.node_ref(false));
		let text_type = self.type_manager.text_type;
		let mut locals = vec![ValType::I64; 8];
		locals.extend([ValType::I32, self.text_ref_nullable()]);
		locals.extend([ValType::I64; 2]);
		self.runtime_function(TIME_TEXT, vec![node_ref], vec![node_ref], locals, |s, f| {
			let (form, days, nanos, year, month, day, fraction, power, position, bytes, offset, instant) =
				(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12);
			let advance = |f: &mut Function, count: i32| {
				Self::emit_list(f, &[I::LocalGet(position), I::I32Const(count), I::I32Add, I::LocalSet(position)])
			};
			let put = |f: &mut Function, byte: u8| {
				Self::emit_list(
					f,
					&[I::LocalGet(bytes), I::LocalGet(position), I::I32Const(byte as i32), I::ArraySet(text_type)],
				);
				advance(f, 1);
			};
			// the value `count` digits wide, with leading zeros
			let digits = |f: &mut Function, value: &[I<'static>], count: u32| {
				for place in 0..count {
					Self::emit_list(
						f,
						&[I::LocalGet(bytes), I::LocalGet(position), I::I32Const(place as i32), I::I32Add],
					);
					Self::emit_list(f, value);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::I64Const(10_i64.pow(count - 1 - place)), I::I64DivU, I::I64Const(10), I::I64RemU,
						I::I64Const(ZERO_DIGIT), I::I64Add, I::I32WrapI64, I::ArraySet(text_type),
					]);
				}
				advance(f, count as i32);
			};
			let copy = |f: &mut Function, literal: Literal| {
				s.emit_literal_into(f, (bytes, position), literal);
				advance(f, literal.1 as i32);
			};
			let is_form = |f: &mut Function, time_form: TimeForm| {
				Self::emit_list(f, &[I::LocalGet(form), I::I64Const(time_form as i64), I::I64Eq])
			};

			s.emit_time_form(f, 0);
			f.instruction(&I::LocalSet(form));
			is_form(f, TimeForm::Duration);
			Self::emit_list(
				f,
				&[I::If(BlockType::Empty), I::LocalGet(0), I::Call(s.func_index(DURATION_TEXT)), I::Return, I::End],
			);
			s.emit_time_position(f, 0);
			f.instruction(&I::LocalSet(instant));
			s.emit_zone_offset(f, 0);
			f.instruction(&I::LocalSet(offset));
			// a zoned time shows its wall clock
			Self::emit_list(f, &[I::LocalGet(form), I::LocalGet(instant)]);
			is_form(f, TimeForm::Zoned);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::If(BlockType::Result(ValType::I64)),
					I::LocalGet(offset), I::I64Const(SECOND as i64), I::I64Mul,
				I::Else,
					I::I64Const(0),
				I::End,
				I::I64Add,
			]);
			s.call(f, TIME_PARTS);
			Self::emit_list(f, &[I::LocalSet(nanos), I::LocalSet(days), I::LocalGet(days)]);
			s.call(f, CIVIL_FROM_DAYS);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalSet(day),
				I::LocalSet(month),
				I::LocalSet(year),
				I::I32Const(TIME_TEXT_BYTES), I::ArrayNewDefault(text_type), I::LocalSet(bytes),
				I::I32Const(0), I::LocalSet(position),
			]);
			// a time of day has no date
			is_form(f, TimeForm::Clock);
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty)]);
			digits(f, &[I::LocalGet(year)], 4);
			put(f, b'-');
			digits(f, &[I::LocalGet(month)], 2);
			put(f, b'-');
			digits(f, &[I::LocalGet(day)], 2);
			f.instruction(&I::End);
			// a date has no clock
			is_form(f, TimeForm::Date);
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty)]);
			is_form(f, TimeForm::Clock);
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty)]);
			put(f, b'T');
			f.instruction(&I::End);
			digits(f, &[I::LocalGet(nanos), I::I64Const(HOUR * SECOND as i64), I::I64DivU], 2);
			put(f, b':');
			digits(
				f,
				&[I::LocalGet(nanos), I::I64Const(MINUTE * SECOND as i64), I::I64DivU, I::I64Const(MINUTE), I::I64RemU],
				2,
			);
			// seconds only when there are any or it is a time of day (22:27:46), the fraction without its trailing zeros
			Self::emit_list(f, &[I::LocalGet(nanos), I::I64Const(SECOND as i64), I::I64RemU, I::LocalSet(fraction)]);
			let whole_seconds =
				[I::LocalGet(nanos), I::I64Const(SECOND as i64), I::I64DivU, I::I64Const(MINUTE), I::I64RemU];
			Self::emit_list(f, &whole_seconds);
			Self::emit_list(f, &[I::LocalGet(fraction), I::I64Or, I::I64Const(0), I::I64Ne]);
			is_form(f, TimeForm::Clock);
			Self::emit_list(f, &[I::I32Or, I::If(BlockType::Empty)]);
			put(f, b':');
			digits(f, &whole_seconds, 2);
			f.instruction(&I::End);
			s.emit_fraction(f, (bytes, position), fraction, power);
			is_form(f, TimeForm::Instant);
			f.instruction(&I::If(BlockType::Empty));
			put(f, b'Z');
			f.instruction(&I::End);
			if let Some((names, prediction)) = &zone_names {
				// `+01:00[Europe/Berlin]`, then the rules version of a prediction, as calendar.rs Zoned::plain
				is_form(f, TimeForm::Zoned);
				Self::emit_list(
					f,
					&[I::If(BlockType::Empty), I::LocalGet(offset), I::I64Const(0), I::I64LtS, I::If(BlockType::Empty)],
				);
				put(f, b'-');
				Self::emit_list(f, &[I::I64Const(0), I::LocalGet(offset), I::I64Sub, I::LocalSet(offset), I::Else]);
				put(f, b'+');
				f.instruction(&I::End);
				digits(f, &[I::LocalGet(offset), I::I64Const(HOUR), I::I64DivU], 2);
				put(f, b':');
				digits(f, &[I::LocalGet(offset), I::I64Const(MINUTE), I::I64DivU, I::I64Const(MINUTE), I::I64RemU], 2);
				for (index, name) in names.iter().enumerate() {
					s.emit_field(f, 0, 0);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::I64Const(KIND_BITS + TIME_FORM_BITS), I::I64ShrU, I::I64Const(ZONE_INDEX_MASK), I::I64And,
						I::I64Const(index as i64), I::I64Eq, I::If(BlockType::Empty),
					]);
					copy(f, *name);
					f.instruction(&I::End);
				}
				let published = BUILTIN_RULES.published.days() * DAY_NANOS;
				Self::emit_list(f, &[I::LocalGet(instant), I::I64Const(published), I::I64GeS, I::If(BlockType::Empty)]);
				copy(f, *prediction);
				Self::emit_list(f, &[I::End, I::End]);
			}
			f.instruction(&I::End);
			s.emit_text_prefix(f, bytes, position);
		});
	}

	/// `.5` of the nanoseconds in `fraction`, without trailing zeros, written into the $Text in local `bytes` at
	/// `position`, which it advances
	fn emit_fraction(&self, func: &mut Function, (bytes, position): (u32, u32), fraction: u32, power: u32) {
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::LocalGet(fraction), I::I64Const(0), I::I64Ne, I::If(BlockType::Empty),
				I::I64Const(SECOND as i64), I::LocalSet(power),
		]);
		#[rustfmt::skip]
		Self::emit_while(func, &[I::LocalGet(fraction), I::I64Const(10), I::I64RemU, I::I64Eqz], &[
			I::LocalGet(fraction), I::I64Const(10), I::I64DivU, I::LocalSet(fraction),
			I::LocalGet(power), I::I64Const(10), I::I64DivU, I::LocalSet(power),
		]);
		// power + fraction keeps the fraction's leading zeros; the point overwrites its leading 1
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::LocalGet(bytes), I::LocalGet(position), I::LocalGet(power), I::LocalGet(fraction), I::I64Add,
		]);
		self.call(func, "int_to_decimal");
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::LocalGet(bytes), I::LocalGet(position), I::I32Const(b'.' as i32), I::ArraySet(self.type_manager.text_type),
			I::LocalGet(position), I::I32Add, I::LocalSet(position),
			I::End,
		]);
	}

	/// duration_text(node): its months, days, then hours, minutes and seconds of whole seconds, else the seconds with
	/// their fraction, each amount with its unit, as calendar.rs `Display for Duration`
	fn emit_duration_text(&mut self) {
		let node_ref = ValType::Ref(self.node_ref(false));
		let text_type = self.type_manager.text_type;
		let mut locals = vec![ValType::I64; 5];
		locals.extend([ValType::I32, self.text_ref_nullable()]);
		let units =
			["month", "day", "hour", "minute", "second"].map(|unit| self.string_table.literal(&format!(" {unit}")));
		self.runtime_function(DURATION_TEXT, vec![node_ref], vec![node_ref], locals, |s, f| {
			let (nanos, seconds, amount, fraction, power, position, bytes) = (1, 2, 3, 4, 5, 6, 7);
			let advance = |f: &mut Function, count: &[I<'static>]| {
				Self::emit_list(f, &[I::LocalGet(position)]);
				Self::emit_list(f, count);
				Self::emit_list(f, &[I::I32Add, I::LocalSet(position)]);
			};
			let put = |f: &mut Function, byte: u8| {
				Self::emit_list(
					f,
					&[I::LocalGet(bytes), I::LocalGet(position), I::I32Const(byte as i32), I::ArraySet(text_type)],
				);
				advance(f, &[I::I32Const(1)]);
			};
			let number = |f: &mut Function, value: &[I<'static>]| {
				let mut written = vec![I::LocalGet(bytes), I::LocalGet(position)];
				written.extend_from_slice(value);
				written.push(I::Call(s.func_index("int_to_decimal")));
				advance(f, &written);
			};
			let unit = |f: &mut Function, literal: Literal| {
				s.emit_literal_into(f, (bytes, position), literal);
				advance(f, &[I::I32Const(literal.1 as i32)]);
			};
			// `amount unit` and a space, the unit plural unless the amount is 1
			let part = |f: &mut Function, value: &[I<'static>], name: Literal| {
				Self::emit_list(f, value);
				Self::emit_list(f, &[I::LocalTee(amount), I::I64Const(0), I::I64Ne, I::If(BlockType::Empty)]);
				number(f, &[I::LocalGet(amount)]);
				unit(f, name);
				Self::emit_list(f, &[I::LocalGet(amount), I::I64Const(1), I::I64Ne, I::If(BlockType::Empty)]);
				put(f, b's');
				f.instruction(&I::End);
				put(f, b' ');
				f.instruction(&I::End);
			};
			let [months, days, hours, minutes, whole_seconds] = units;
			s.emit_time_position(f, 0);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalSet(nanos),
				I::I32Const(DURATION_TEXT_BYTES), I::ArrayNewDefault(text_type), I::LocalSet(bytes),
			]);
			let info = |shift: i64| {
				[
					I::LocalGet(0),
					I::RefAsNonNull,
					I::StructGet { struct_type_index: s.type_manager.node_type, field_index: 0 },
					I::I64Const(KIND_BITS + shift),
					I::I64ShrS,
				]
			};
			part(f, &info(MONTHS_SHIFT), months);
			let mut day_count = info(TIME_FORM_BITS).to_vec();
			day_count.extend([I::I32WrapI64, I::I64ExtendI32S]);
			part(f, &day_count, days);
			// whole seconds read as hours, minutes and seconds: `3 hours`, not `10800 seconds`
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(nanos), I::I64Const(SECOND as i64), I::I64RemS, I::I64Eqz, I::LocalGet(nanos),
				I::I64Const(0), I::I64Ne, I::I32And, I::If(BlockType::Empty),
					I::LocalGet(nanos), I::I64Const(SECOND as i64), I::I64DivS, I::LocalSet(seconds),
			]);
			part(f, &[I::LocalGet(seconds), I::I64Const(HOUR), I::I64DivS], hours);
			part(f, &[I::LocalGet(seconds), I::I64Const(HOUR), I::I64RemS, I::I64Const(MINUTE), I::I64DivS], minutes);
			part(f, &[I::LocalGet(seconds), I::I64Const(MINUTE), I::I64RemS], whole_seconds);
			f.instruction(&I::Else);
			// `1.5 seconds`, `0 seconds` for no duration at all
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(nanos), I::I64Const(0), I::I64Ne, I::LocalGet(position), I::I32Eqz, I::I32Or,
				I::If(BlockType::Empty),
					I::LocalGet(nanos), I::I64Const(0), I::I64LtS, I::If(BlockType::Empty),
			]);
			put(f, b'-');
			Self::emit_list(f, &[I::I64Const(0), I::LocalGet(nanos), I::I64Sub, I::LocalSet(nanos), I::End]);
			number(f, &[I::LocalGet(nanos), I::I64Const(SECOND as i64), I::I64DivU]);
			Self::emit_list(f, &[I::LocalGet(nanos), I::I64Const(SECOND as i64), I::I64RemU, I::LocalSet(fraction)]);
			s.emit_fraction(f, (bytes, position), fraction, power);
			unit(f, whole_seconds);
			put(f, b's');
			put(f, b' ');
			Self::emit_list(f, &[I::End, I::End]);
			// without the last space
			advance(f, &[I::I32Const(-1)]);
			s.emit_text_prefix(f, bytes, position);
		});
	}

	/// time_of(info, position) of two constants: the node itself
	pub(in crate::wasm_emitter) fn emit_time_constant(&self, func: &mut Function, info: i64, position: i64) {
		Self::emit_list(func, &[I::I64Const((info << KIND_BITS) | Kind::Time as i64), I::I64Const(position)]);
		self.emit_time_node_of_position(func);
	}

	/// The time node of the kind and the position on the stack
	fn emit_time_node_of_position(&self, func: &mut Function) {
		let (node, i64_box) = (self.type_manager.node_type, self.type_manager.i64_box_type);
		Self::emit_list(func, &[I::StructNew(i64_box), I::RefNull(HeapType::Concrete(node)), I::StructNew(node)]);
	}

	/// Two times in node_order, whose locals `kinds` hold their kinds: the earlier one first; only the same form compares
	pub(in crate::wasm_emitter) fn emit_time_order(
		&mut self,
		func: &mut Function,
		kinds: [u32; 2],
		first: u32,
		second: u32,
	) {
		// required, not merely emitted: emit_all_functions builds node_order without times, its refusal would mark the
		// text constructors used (test_function_usage_tracking)
		if !self.ctx.required_functions.contains(TIME_FIELD) {
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
		self.emit_time_form(func, first);
		Self::emit_list(func, &[I::I64Const(TimeForm::Duration as i64), I::I64Eq, I::If(BlockType::Empty)]);
		for duration in [first, second] {
			self.emit_field(func, duration, 0);
			self.emit_time_position(func, duration);
		}
		self.call(func, DURATION_ORDER);
		Self::emit_list(func, &[I::Return, I::End]);
		for comparison in [I::I64GtS, I::I64LtS] {
			self.emit_time_position(func, first);
			self.emit_time_position(func, second);
			func.instruction(&comparison);
		}
		Self::emit_list(func, &[I::I32Sub, I::Return, I::End]);
	}

	/// `t.month` of a time: its field, or the refusal of a form without it (`instant has no hour`, `date has no hour`)
	pub(in crate::wasm_emitter) fn emit_time_field_lookup(&mut self, func: &mut Function, held: u32, name: &str) {
		if name == TZDATA_FIELD {
			return self.emit_tzdata_lookup(func, held);
		}
		if !self.should_emit_function(TIME_FIELD) {
			return;
		}
		let Some(field) = TIME_FIELDS.iter().position(|known| *known == name) else {
			return;
		};
		self.emit_field(func, held, 0);
		Self::emit_kind_test(func, Kind::Time as i64, I::I64Eq);
		Self::emit_list(func, &[I::If(BlockType::Empty)]);
		for form in TIME_FORMS {
			if let Some(refusal) = form.refusal(name) {
				self.emit_time_form(func, held);
				Self::emit_list(func, &[I::I64Const(form as i64), I::I64Eq, I::If(BlockType::Empty)]);
				self.emit_trap_detail(func, &Node::Text(refusal));
				self.emit_runtime_error(func, RETURNED_ERROR);
				func.instruction(&I::End);
			}
		}
		let part_form = TIME_PARTS_FORMS.iter().find(|(part, _)| *part == name).map(|(_, form)| *form);
		if let Some(form) = part_form {
			func.instruction(&I::I64Const(((form as i64) << KIND_BITS) | Kind::Time as i64));
		}
		Self::emit_list(func, &[I::LocalGet(held), I::RefAsNonNull, I::I32Const(field as i32)]);
		self.call(func, TIME_FIELD);
		match part_form {
			Some(_) => self.emit_time_node_of_position(func),
			None => self.call(func, "new_int"),
		}
		Self::emit_list(func, &[I::Br(1), I::End]);
	}

	/// `t.tzdata` of a zoned time: the version of the rules it was made with, those of the compilation
	fn emit_tzdata_lookup(&mut self, func: &mut Function, held: u32) {
		self.emit_field(func, held, 0);
		Self::emit_kind_test(func, Kind::Time as i64, I::I64Eq);
		Self::emit_list(func, &[I::If(BlockType::Empty)]);
		self.emit_time_form(func, held);
		Self::emit_list(func, &[I::I64Const(TimeForm::Zoned as i64), I::I64Eq, I::If(BlockType::Empty)]);
		self.emit_node_instructions(func, &Node::Text(crate::time::calendar::rules().version.to_string()));
		Self::emit_list(func, &[I::Br(2), I::End, I::End]);
	}

	/// The TimeForm of the time node in `local`
	pub(in crate::wasm_emitter) fn emit_time_form(&self, func: &mut Function, local: u32) {
		self.emit_field(func, local, 0);
		Self::emit_list(func, &[I::I64Const(KIND_BITS), I::I64ShrU, I::I64Const(TIME_FORM_MASK), I::I64And]);
	}

	/// The offset of the zoned time node in `local`, seconds east of UTC
	fn emit_zone_offset(&self, func: &mut Function, local: u32) {
		self.emit_field(func, local, 0);
		Self::emit_list(func, &[I::I64Const(KIND_BITS + OFFSET_SHIFT), I::I64ShrS, I::I64Const(MINUTE), I::I64Mul]);
	}

	/// The `[Zone/Name]` of every zone and the rules suffix of a prediction, in linear memory, when a zoned time is shown
	fn zone_names(&mut self) -> Option<(Vec<Literal>, Literal)> {
		if !self.should_emit_function(ZONE_NAMES) {
			return None;
		}
		let names = ZONES.iter().map(|zone| self.string_table.literal(&format!("[{}]", zone.name))).collect();
		Some((names, self.string_table.literal(&format!("[_tzdata={}]", BUILTIN_RULES.version))))
	}

	/// The position of the time node in `local`
	pub(in crate::wasm_emitter) fn emit_time_position(&self, func: &mut Function, local: u32) {
		let i64_box = self.type_manager.i64_box_type;
		self.emit_field(func, local, 1);
		Self::emit_list(
			func,
			&[
				I::RefCastNonNull(HeapType::Concrete(i64_box)),
				I::StructGet { struct_type_index: i64_box, field_index: 0 },
			],
		);
	}

	/// Quotient and remainder rounded toward minus infinity, as div_euclid and rem_euclid
	pub(in crate::wasm_emitter) fn emit_floor_split(
		func: &mut Function,
		dividend: u32,
		divisor: i64,
		quotient: u32,
		remainder: u32,
	) {
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::LocalGet(dividend), I::I64Const(divisor), I::I64DivS, I::LocalSet(quotient),
			I::LocalGet(dividend), I::LocalGet(quotient), I::I64Const(divisor), I::I64Mul, I::I64Sub,
			I::LocalTee(remainder), I::I64Const(0), I::I64LtS, I::If(BlockType::Empty),
				I::LocalGet(remainder), I::I64Const(divisor), I::I64Add, I::LocalSet(remainder),
				I::LocalGet(quotient), I::I64Const(1), I::I64Sub, I::LocalSet(quotient),
			I::End,
		]);
	}
}

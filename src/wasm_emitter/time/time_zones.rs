//! Time zones at run time (card run-time-dates stage B3, notes/dates_at_run_time.md): the offset a zone has at an
//! instant under the Europe and United States daylight saving rules, resolving a wall time that a transition skips or
//! repeats, and `t in "Zone"`, as calendar.rs `Zone::offset_at`, `Zone::resolve` and `Time::in_zone` do for constants.

use super::super::time::times::{CIVIL_FROM_DAYS, DAYS_FROM_CIVIL};
use super::super::WasmGcEmitter;
use crate::time::calendar::{Dst, DAY_SECONDS, HOUR, MINUTE, SECOND, ZONES};
use crate::time::{TimeForm, OFFSET_SHIFT, TIME_FORM_BITS};
use crate::type_kinds::{Kind, KIND_BITS};
use wasm_encoder::*;
use Instruction as I;

/// zone_rule(zone) -> (seconds east of UTC in winter, the Dst of the zone)
const ZONE_RULE: &str = "zone_rule";
/// zone_offset_at(zone, seconds since 1970) -> seconds east of UTC
const ZONE_OFFSET_AT: &str = "zone_offset_at";
/// zone_resolve(zone, wall nanoseconds, choice) -> (instant nanoseconds, status): the instant a wall time means
pub(in crate::wasm_emitter) const ZONE_RESOLVE: &str = "zone_resolve";
/// zoned_node(zone, instant nanoseconds) -> a zoned time node
pub(in crate::wasm_emitter) const ZONED_NODE: &str = "zoned_node";
/// time_in_zone(time, zone, choice) -> zoned time: `t in "Europe/Berlin"`, `zoned(t, "Europe/Berlin", disambiguation: later)`
pub const TIME_IN_ZONE: &str = "time_in_zone";

/// The status of zone_resolve
pub(in crate::wasm_emitter) const RESOLVED: i64 = 0;
const SKIPPED: i64 = 1;
const AMBIGUOUS: i64 = 2;
/// The choice of zone_resolve, as the lowering writes it (time.rs `choice_code`)
pub const REJECT: i64 = 0;
pub const EARLIER: i64 = 1;
pub const LATER: i64 = 2;

const DST_NONE: i64 = 0;
const DST_EUROPE: i64 = 1;
const DST_UNITED_STATES: i64 = 2;
const MARCH: i64 = 3;
const OCTOBER: i64 = 10;
const NOVEMBER: i64 = 11;
const DAYS_PER_WEEK: i64 = 7;
const NANOS_PER_SECOND: i64 = SECOND as i64;
const SKIPPED_MESSAGE: &str = "that wall time does not exist in the zone: it is skipped by a daylight saving transition; choose disambiguation: earlier or later";
const AMBIGUOUS_MESSAGE: &str = "that wall time is ambiguous in the zone: it occurs twice, the clocks are set back by a daylight saving transition; choose disambiguation: earlier or later";
const NOT_A_TIME: &str = "`in` places a time in a zone";
const NO_CLOCK: &str = "a date has no time of day: give it a clock before placing it in a zone";

type Emit = Vec<I<'static>>;

fn dst_code(dst: Dst) -> i64 {
	match dst {
		Dst::None => DST_NONE,
		Dst::Europe => DST_EUROPE,
		Dst::UnitedStates => DST_UNITED_STATES,
	}
}

/// ISO weekday (Monday is 1) of the day count in `days`, as calendar.rs weekday_of
fn weekday_of(days: u32) -> Emit {
	vec![
		I::LocalGet(days),
		I::I64Const(3),
		I::I64Add,
		I::I64Const(DAYS_PER_WEEK),
		I::I64RemS,
		I::I64Const(DAYS_PER_WEEK),
		I::I64Add,
		I::I64Const(DAYS_PER_WEEK),
		I::I64RemS,
		I::I64Const(1),
		I::I64Add,
	]
}

impl WasmGcEmitter {
	/// The zone functions, when a program makes a zoned time; after emit_times_runtime, whose calendar they use
	pub(in crate::wasm_emitter) fn emit_time_zones(&mut self) {
		// required, not merely emitted, as time arithmetic (uses_times): their refusals mark the text constructors used
		if !self.uses_times() || !self.ctx.required_functions.contains(super::super::time::times::ZONE_NAMES) {
			return;
		}
		self.emit_zone_rule();
		self.emit_zone_offset_at();
		self.emit_zone_resolve();
		self.emit_zoned_node();
		self.emit_time_in_zone();
	}

	fn emit_zone_rule(&mut self) {
		self.runtime_function(ZONE_RULE, vec![ValType::I64], vec![ValType::I64; 2], vec![ValType::I64; 2], |_, f| {
			let (zone, standard, dst) = (0, 1, 2);
			for (index, known) in ZONES.iter().enumerate() {
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(zone), I::I64Const(index as i64), I::I64Eq, I::If(BlockType::Empty),
						I::I64Const(known.standard), I::LocalSet(standard),
						I::I64Const(dst_code(known.dst)), I::LocalSet(dst),
					I::End,
				]);
			}
			Self::emit_list(f, &[I::LocalGet(standard), I::LocalGet(dst)]);
		});
	}

	/// The Sunday on or before the last day of `month`, as calendar.rs Date::last_sunday; the day count on the stack
	fn emit_last_sunday(&self, f: &mut Function, year: u32, month: i64, last: u32) {
		Self::emit_list(f, &[I::LocalGet(year), I::I64Const(month + 1), I::I64Const(1)]);
		self.call(f, DAYS_FROM_CIVIL);
		Self::emit_list(f, &[I::I64Const(1), I::I64Sub, I::LocalSet(last), I::LocalGet(last)]);
		Self::emit_list(f, &weekday_of(last));
		Self::emit_list(f, &[I::I64Const(DAYS_PER_WEEK), I::I64RemS, I::I64Sub]);
	}

	/// The n-th Sunday of `month`, counting from 1, as calendar.rs Date::nth_sunday; the day count on the stack
	fn emit_nth_sunday(&self, f: &mut Function, year: u32, month: i64, nth: i64, first: u32) {
		Self::emit_list(f, &[I::LocalGet(year), I::I64Const(month), I::I64Const(1)]);
		self.call(f, DAYS_FROM_CIVIL);
		Self::emit_list(f, &[I::LocalSet(first), I::LocalGet(first), I::I64Const(DAYS_PER_WEEK)]);
		Self::emit_list(f, &weekday_of(first));
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::I64Sub, I::I64Const(DAYS_PER_WEEK), I::I64RemS, I::I64Add, I::I64Const(DAYS_PER_WEEK * (nth - 1)),
			I::I64Add,
		]);
	}

	fn emit_zone_offset_at(&mut self) {
		self.runtime_function(
			ZONE_OFFSET_AT,
			vec![ValType::I64; 2],
			vec![ValType::I64],
			vec![ValType::I64; 8],
			|s, f| {
				let (zone, seconds, standard, dst, days, spare, year, start, end) = (0, 1, 2, 3, 4, 5, 6, 7, 8);
				Self::emit_list(f, &[I::LocalGet(zone)]);
				s.call(f, ZONE_RULE);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalSet(dst),
					I::LocalSet(standard),
					I::LocalGet(dst), I::I64Const(DST_NONE), I::I64Eq, I::If(BlockType::Empty),
						I::LocalGet(standard), I::Return,
					I::End,
				]);
				Self::emit_floor_split(f, seconds, DAY_SECONDS, days, spare);
				Self::emit_list(f, &[I::LocalGet(days)]);
				s.call(f, CIVIL_FROM_DAYS);
				Self::emit_list(f, &[I::Drop, I::Drop, I::LocalSet(year)]);
				let day_seconds = |f: &mut Function| Self::emit_list(f, &[I::I64Const(DAY_SECONDS), I::I64Mul]);
				Self::emit_list(f, &[I::LocalGet(dst), I::I64Const(DST_EUROPE), I::I64Eq, I::If(BlockType::Empty)]);
				// last Sunday of March to last Sunday of October, switching at 01:00 UTC
				s.emit_last_sunday(f, year, MARCH, spare);
				day_seconds(f);
				Self::emit_list(f, &[I::I64Const(HOUR), I::I64Add, I::LocalSet(start)]);
				s.emit_last_sunday(f, year, OCTOBER, spare);
				day_seconds(f);
				Self::emit_list(f, &[I::I64Const(HOUR), I::I64Add, I::LocalSet(end), I::Else]);
				// second Sunday of March to first Sunday of November, switching at 02:00 local time
				s.emit_nth_sunday(f, year, MARCH, 2, spare);
				day_seconds(f);
				Self::emit_list(
					f,
					&[I::I64Const(2 * HOUR), I::I64Add, I::LocalGet(standard), I::I64Sub, I::LocalSet(start)],
				);
				s.emit_nth_sunday(f, year, NOVEMBER, 1, spare);
				day_seconds(f);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::I64Const(2 * HOUR), I::I64Add, I::LocalGet(standard), I::I64Sub, I::I64Const(HOUR), I::I64Sub,
					I::LocalSet(end),
					I::End,
					I::LocalGet(standard), I::I64Const(HOUR), I::I64Add, I::LocalGet(standard), I::LocalGet(seconds),
					I::LocalGet(start), I::I64GeS, I::LocalGet(seconds), I::LocalGet(end), I::I64LtS, I::I32And,
					I::Select,
				]);
			},
		);
	}

	/// calendar.rs `Zone::resolve`: of the two offsets of a zone, the ones that hold at the instant they give; one is
	/// the answer, none (skipped) or two (repeated) need the choice
	fn emit_zone_resolve(&mut self) {
		self.runtime_function(
			ZONE_RESOLVE,
			vec![ValType::I64; 3],
			vec![ValType::I64; 2],
			vec![ValType::I64; 8],
			|s, f| {
				let (zone, wall, choice, standard, dst, earlier, later, valid_earlier, valid_later, spare) =
					(0, 1, 2, 3, 4, 5, 6, 7, 8, 9);
				Self::emit_list(f, &[I::LocalGet(zone)]);
				s.call(f, ZONE_RULE);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalSet(dst),
					I::LocalSet(standard),
					I::LocalGet(dst), I::I64Const(DST_NONE), I::I64Eq, I::If(BlockType::Empty),
						I::LocalGet(wall), I::LocalGet(standard), I::I64Const(NANOS_PER_SECOND), I::I64Mul, I::I64Sub,
						I::I64Const(RESOLVED), I::Return,
					I::End,
				]);
				// the instants of the wall time under the summer offset (the earlier) and the winter offset (the later)
				for (instant, extra) in [(earlier, HOUR), (later, 0)] {
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(wall), I::LocalGet(standard), I::I64Const(extra), I::I64Add,
						I::I64Const(NANOS_PER_SECOND), I::I64Mul, I::I64Sub, I::LocalSet(instant),
					]);
				}
				for (instant, valid, extra) in [(earlier, valid_earlier, HOUR), (later, valid_later, 0)] {
					Self::emit_floor_split(f, instant, NANOS_PER_SECOND, spare, spare + 1);
					Self::emit_list(f, &[I::LocalGet(zone), I::LocalGet(spare)]);
					s.call(f, ZONE_OFFSET_AT);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(standard), I::I64Const(extra), I::I64Add, I::I64Eq, I::I64ExtendI32U,
						I::LocalSet(valid),
					]);
				}
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(valid_earlier), I::LocalGet(valid_later), I::I64Xor, I::I32WrapI64,
					I::If(BlockType::Empty),
						I::LocalGet(earlier), I::LocalGet(later), I::LocalGet(valid_earlier), I::I32WrapI64, I::Select,
						I::I64Const(RESOLVED), I::Return,
					I::End,
					I::LocalGet(choice), I::I64Const(EARLIER), I::I64Eq, I::If(BlockType::Empty),
						I::LocalGet(earlier), I::I64Const(RESOLVED), I::Return,
					I::End,
					I::LocalGet(choice), I::I64Const(LATER), I::I64Eq, I::If(BlockType::Empty),
						I::LocalGet(later), I::I64Const(RESOLVED), I::Return,
					I::End,
					I::LocalGet(earlier), I::I64Const(AMBIGUOUS), I::I64Const(SKIPPED), I::LocalGet(valid_earlier),
					I::I32WrapI64, I::Select,
				]);
			},
		);
	}

	/// The zoned time at an instant: its zone and offset in the info bits, as time.rs `time_info`
	fn emit_zoned_node(&mut self) {
		let node_ref = ValType::Ref(self.node_ref(false));
		self.runtime_function(ZONED_NODE, vec![ValType::I64; 2], vec![node_ref], vec![ValType::I64; 2], |s, f| {
			let (zone, instant, kind, spare) = (0, 1, 2, 3);
			let (node, i64_box) = (s.type_manager.node_type, s.type_manager.i64_box_type);
			Self::emit_floor_split(f, instant, NANOS_PER_SECOND, kind, spare);
			Self::emit_list(f, &[I::LocalGet(zone), I::LocalGet(kind)]);
			s.call(f, ZONE_OFFSET_AT);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I64Const(MINUTE), I::I64DivS, I::I64Const(KIND_BITS + OFFSET_SHIFT), I::I64Shl, I::LocalGet(zone),
				I::I64Const(KIND_BITS + TIME_FORM_BITS), I::I64Shl, I::I64Or,
				I::I64Const(((TimeForm::Zoned as i64) << KIND_BITS) | Kind::Time as i64), I::I64Or,
				I::LocalGet(instant), I::StructNew(i64_box), I::RefNull(HeapType::Concrete(node)), I::StructNew(node),
			]);
		});
	}

	fn emit_time_in_zone(&mut self) {
		let node_ref = ValType::Ref(self.node_ref(false));
		self.runtime_function(
			TIME_IN_ZONE,
			vec![node_ref, ValType::I64, ValType::I64],
			vec![node_ref],
			vec![ValType::I64; 3],
			|s, f| {
				let (time, zone, choice, form, position, status) = (0, 1, 2, 3, 4, 5);
				s.emit_field(f, time, 0);
				Self::emit_kind_test(f, Kind::Time as i64, I::I64Ne);
				Self::emit_list(f, &[I::If(BlockType::Empty)]);
				s.emit_time_failure(f, NOT_A_TIME);
				f.instruction(&I::End);
				s.emit_time_form(f, time);
				f.instruction(&I::LocalSet(form));
				s.emit_time_position(f, time);
				f.instruction(&I::LocalSet(position));
				// an instant and a zoned time keep their instant: only the wall clock changes
				let is_form = |form_local: u32, wanted: TimeForm| {
					vec![I::LocalGet(form_local), I::I64Const(wanted as i64), I::I64Eq]
				};
				Self::emit_list(
					f,
					&[
						is_form(form, TimeForm::Instant),
						is_form(form, TimeForm::Zoned),
						vec![I::I32Or, I::If(BlockType::Empty)],
					]
					.concat(),
				);
				Self::emit_list(f, &[I::LocalGet(zone), I::LocalGet(position)]);
				s.call(f, ZONED_NODE);
				Self::emit_list(f, &[I::Return, I::End]);
				Self::emit_list(
					f,
					&[is_form(form, TimeForm::Local), vec![I::I32Eqz, I::If(BlockType::Empty)]].concat(),
				);
				s.emit_time_failure(f, NO_CLOCK);
				f.instruction(&I::End);
				s.emit_resolved_instant(f, zone, position, choice, status);
				Self::emit_list(f, &[I::LocalGet(zone), I::LocalGet(position)]);
				s.call(f, ZONED_NODE);
			},
		);
	}

	/// Puts the instant of the wall time in `wall` into it, resolved in `zone` with `choice`; a skipped or repeated
	/// wall time with no choice fails
	pub(in crate::wasm_emitter) fn emit_resolved_instant(
		&mut self,
		f: &mut Function,
		zone: u32,
		wall: u32,
		choice: u32,
		status: u32,
	) {
		Self::emit_list(f, &[I::LocalGet(zone), I::LocalGet(wall), I::LocalGet(choice)]);
		self.call(f, ZONE_RESOLVE);
		Self::emit_list(f, &[I::LocalSet(status), I::LocalSet(wall)]);
		for (code, message) in [(SKIPPED, SKIPPED_MESSAGE), (AMBIGUOUS, AMBIGUOUS_MESSAGE)] {
			Self::emit_list(f, &[I::LocalGet(status), I::I64Const(code), I::I64Eq, I::If(BlockType::Empty)]);
			self.emit_time_failure(f, message);
			f.instruction(&I::End);
		}
	}
}

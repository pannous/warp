//! card time-arithmetic-warp: the run-time arithmetic of times and durations in lib/compiler/time_arithmetic.warp
//! (Rust twin: src/wasm_emitter/time/time_arithmetic.rs) over plain integers, before card time-primitives connects it
//! to run-time time nodes
use crate::is;

fn arithmetic(code: &str) -> String {
	format!("use \"lib/compiler/time_arithmetic.warp\"\n{code}")
}

fn date(code: &str) -> String {
	arithmetic(&format!("start = Moment(TimeForm::Date, days_from_date(2024, 1, 15))\n{code}"))
}

#[test]
fn dates_shift_by_months_then_days() {
	is!(&date("shift_time(start, Span(3, 0, 0)).position == days_from_date(2024, 4, 15)"), true);
	is!(&date("shift_time(start, Span(1, 2, 0)).position == days_from_date(2024, 2, 17)"), true);
	is!(&date("shift_time(start, Span(-1, 0, 0)).position == days_from_date(2023, 12, 15)"), true);
	is!(&date("shift_time(start, Span(12, -15, 0)).position == days_from_date(2024, 12, 31)"), true);
}

#[test]
fn a_missing_day_is_refused() {
	is!(
		&arithmetic("start = Moment(TimeForm::Date, days_from_date(2024, 1, 31))\ntry { shift_time(start, Span(1, 0, 0)) } catch e { e == no_such_day }"),
		true
	);
	is!(&date("try { shift_time(start, Span(0, 0, hour_nanos)) } catch e { e == no_time_of_day }"), true);
}

#[test]
fn local_times_keep_their_clock() {
	is!(
		&arithmetic(
			"t = Moment(TimeForm::Local, days_from_date(2024, 1, 31) * day_nanos + 12 * hour_nanos)\nshift_time(t, Span(0, 1, 0)).position == days_from_date(2024, 2, 1) * day_nanos + 12 * hour_nanos"
		),
		true
	);
}

#[test]
fn clocks_go_round_and_instants_have_no_calendar() {
	is!(
		&arithmetic(
			"shift_time(Moment(TimeForm::Clock, 23 * hour_nanos), Span(0, 0, 2 * hour_nanos)).position == hour_nanos"
		),
		true
	);
	is!(
		&arithmetic("try { shift_time(Moment(TimeForm::Instant, 0), Span(0, 1, 0)) } catch e { e == no_calendar }"),
		true
	);
}

#[test]
fn operands_swap_scale_and_subtract() {
	is!(
		&arithmetic("r = time_arithmetic(Operand::Whole(3), Operand::Lasting(Span(0, 0, hour_nanos)), TimeOperator::Mul)\nmatch r { Lasting(span) => span.nano_count == 3 * hour_nanos; _ => false }"),
		true
	);
	is!(
		&date("r = time_arithmetic(Operand::At(start), Operand::At(Moment(TimeForm::Date, days_from_date(2024, 1, 1))), TimeOperator::Sub)\nmatch r { Whole(n) => n; _ => 0 }"),
		14
	);
	is!(
		&date("r = time_arithmetic(Operand::Lasting(Span(0, 1, 0)), Operand::At(start), TimeOperator::Add)\nmatch r { At(moment) => moment.position - start.position; _ => 0 }"),
		1
	);
	is!(&date("try { time_arithmetic(Operand::At(start), Operand::At(start), TimeOperator::Add) } catch e { e == not_arithmetic }"), true);
}

#[test]
fn durations_order_when_every_date_agrees() {
	is!(&arithmetic("duration_order(Span(1, 0, 0), Span(0, 27, 0))"), 1);
	is!(&arithmetic("duration_order(Span(0, 0, 3), Span(0, 0, 5))"), -1);
	is!(&arithmetic("try { duration_order(Span(1, 0, 0), Span(0, 30, 0)) } catch e { e == lengths_vary }"), true);
}

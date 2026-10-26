//! Dates and times (Footguns.md → Dates and time zones, notes/dates_at_run_time.md).
//! The parser lexes RFC 3339 / RFC 9557 literals and `1 month` style durations into data nodes. A program this
//! evaluator answers whole is answered here at compile time; in any other its constant time expressions fold to
//! `time_of(form, position)`, a Kind::Time node built when the program runs (wasm_emitter/times.rs), and `now` is
//! `instant_at(clock())` (system_values).

pub mod calendar;

use crate::extensions::numbers::Number;
use crate::node::{error, Node};
pub use calendar::{with_rules, Disambiguation, Duration, Overflow, Time, TzRules, Zone, Zoned};
pub use calendar::{literal_len, unit_after};

/// `now.epoch_seconds`: an instant's whole seconds since 1970
pub const EPOCH_SECONDS: &str = "epoch_seconds";

/// The local time zone's offset at an instant, in nanoseconds
pub fn local_offset(instant: calendar::Instant) -> i64 {
	#[cfg(feature = "native")]
	return warp_runtime::system_signals::local_offset_at(instant.div_euclid(calendar::SECOND / 1000) as i64);
	#[allow(unreachable_code)]
	{
		let _ = instant;
		0
	}
}

/// The form of a run-time time node, in the info bits above Kind::Time; its $i64box holds the position: days since
/// 1970 for a date, nanoseconds since 1970 for the others (a local time as if it were UTC), the exact nanoseconds of a
/// duration. Above the form, a duration keeps its days and months, a zoned time its zone and offset (`time_info`)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimeForm {
	Instant = 0,
	Date = 1,
	Local = 2,
	Duration = 3,
	Zoned = 4,
	/// `now.time`: nanoseconds since midnight
	Clock = 5,
}
pub const TIME_FORMS: [TimeForm; 6] =
	[TimeForm::Instant, TimeForm::Date, TimeForm::Local, TimeForm::Duration, TimeForm::Zoned, TimeForm::Clock];
/// The info bits of the form
pub const TIME_FORM_MASK: i64 = 7;
/// The type of every time, covering the type of each form (card anchor-time): `now is time`, `now is instant`
pub const TIME_WORD: &str = "time";
pub const TIME_FORM_BITS: i64 = 3;
/// A duration's months above its days, which take 32 bits
pub const MONTHS_SHIFT: i64 = TIME_FORM_BITS + 32;
/// A zoned time's index in calendar::ZONES, then its offset in minutes east of UTC
pub const ZONE_INDEX_MASK: i64 = 0xff;
pub const OFFSET_SHIFT: i64 = TIME_FORM_BITS + 8;

fn form_of(info: i64) -> Option<TimeForm> {
	TIME_FORMS.into_iter().find(|form| *form as i64 == info & TIME_FORM_MASK)
}

/// A time read back from the run-time node (Kind::Time): its info bits (TimeForm) and position
pub fn time_node(info: i64, position: Option<Number>) -> Node {
	let Some(Number::Int(position)) = position else {
		return error("unreadable time");
	};
	let position = position as i128;
	match form_of(info) {
		Some(TimeForm::Instant) => Node::data(Time::Instant(position)),
		Some(TimeForm::Date) => Node::data(Time::Date(calendar::civil_from_days(position as i64))),
		Some(TimeForm::Local) => {
			let (date, clock) = calendar::from_wall_nanos(position);
			Node::data(Time::Local(date, clock))
		}
		Some(TimeForm::Clock) => Node::data(Time::Clock(calendar::from_wall_nanos(position).1)),
		Some(TimeForm::Duration) => {
			let days = (info >> TIME_FORM_BITS) as i32 as i64;
			Node::data(Duration { months: info >> MONTHS_SHIFT, days, nanos: position })
		}
		Some(TimeForm::Zoned) => match calendar::ZONES.get(((info >> TIME_FORM_BITS) & ZONE_INDEX_MASK) as usize) {
			Some(zone) => {
				let offset = (info >> OFFSET_SHIFT) * calendar::MINUTE;
				let (date, clock) = calendar::local_at(position, offset);
				Node::data(Time::Zoned(Zoned { date, clock, offset, zone, rules: calendar::rules() }))
			}
			None => error("unreadable time zone"),
		},
		None => error("unreadable time"),
	}
}

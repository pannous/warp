//! Calendar arithmetic without host dependencies: proleptic Gregorian dates, UTC instants and an embedded zone table.
//! Four distinct kinds of time never convert implicitly (Footguns.md → Dates and time zones).

use std::cmp::Ordering;
use std::fmt;

const SECOND: i128 = 1_000_000_000;
const DAY_SECONDS: i64 = 86_400;
const DAY: i128 = DAY_SECONDS as i128 * SECOND;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
	pub year: i64,
	pub month: i64, // 1-based, January is 1
	pub day: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Clock {
	pub hour: i64,
	pub minute: i64,
	pub second: i64,
	pub nanos: i64,
}

/// Nanoseconds since 1970-01-01T00:00Z
pub type Instant = i128;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Time {
	/// `2024-01-31`: a calendar day, no clock, no zone
	Date(Date),
	/// `2024-01-31T10:00`: wall clock without a zone
	Local(Date, Clock),
	/// `2024-01-31T09:00Z`: a point on the UTC line, no calendar fields until placed in a zone
	Instant(Instant),
	/// `2024-01-31T10:00[Europe/Berlin]`: an instant seen through a zone's rules
	Zoned(Instant, &'static Zone),
}

/// Calendar months and days are counted, exact time is measured: `1 day` on a zoned time is a calendar day, `24 hours` is not
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Duration {
	pub months: i64,
	pub days: i64,
	pub nanos: i128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Overflow {
	/// `2024-01-31 + 1 month` is an error naming `2024-02-31`
	Reject,
	/// `add(2024-01-31, 1 month, overflow: clamp)` → `2024-02-29`
	Clamp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dst {
	None,
	/// Last Sunday of March to last Sunday of October, switching at 01:00 UTC (since 1996)
	Europe,
	/// Second Sunday of March to first Sunday of November, switching at 02:00 local time (since 2007)
	UnitedStates,
}

/// Current rules only: dates before the rule's adoption get today's offsets
#[derive(Debug, PartialEq, Eq)]
pub struct Zone {
	pub name: &'static str,
	pub standard: i64, // seconds east of UTC
	pub dst: Dst,
}

const fn zone(name: &'static str, standard: i64, dst: Dst) -> Zone {
	Zone { name, standard, dst }
}

const HOUR: i64 = 3600;

/// Embedded subset of the IANA database; a host import of the full database is future work
pub static ZONES: &[Zone] = &[
	zone("UTC", 0, Dst::None),
	zone("Etc/UTC", 0, Dst::None),
	zone("Europe/London", 0, Dst::Europe),
	zone("Europe/Dublin", 0, Dst::Europe),
	zone("Europe/Lisbon", 0, Dst::Europe),
	zone("Europe/Berlin", HOUR, Dst::Europe),
	zone("Europe/Paris", HOUR, Dst::Europe),
	zone("Europe/Madrid", HOUR, Dst::Europe),
	zone("Europe/Rome", HOUR, Dst::Europe),
	zone("Europe/Amsterdam", HOUR, Dst::Europe),
	zone("Europe/Brussels", HOUR, Dst::Europe),
	zone("Europe/Vienna", HOUR, Dst::Europe),
	zone("Europe/Zurich", HOUR, Dst::Europe),
	zone("Europe/Stockholm", HOUR, Dst::Europe),
	zone("Europe/Oslo", HOUR, Dst::Europe),
	zone("Europe/Copenhagen", HOUR, Dst::Europe),
	zone("Europe/Warsaw", HOUR, Dst::Europe),
	zone("Europe/Prague", HOUR, Dst::Europe),
	zone("Europe/Budapest", HOUR, Dst::Europe),
	zone("Europe/Athens", 2 * HOUR, Dst::Europe),
	zone("Europe/Helsinki", 2 * HOUR, Dst::Europe),
	zone("Europe/Kyiv", 2 * HOUR, Dst::Europe),
	zone("Europe/Bucharest", 2 * HOUR, Dst::Europe),
	zone("Europe/Istanbul", 3 * HOUR, Dst::None),
	zone("Europe/Moscow", 3 * HOUR, Dst::None),
	zone("Asia/Dubai", 4 * HOUR, Dst::None),
	zone("Asia/Kolkata", 5 * HOUR + 1800, Dst::None),
	zone("Asia/Shanghai", 8 * HOUR, Dst::None),
	zone("Asia/Singapore", 8 * HOUR, Dst::None),
	zone("Asia/Tokyo", 9 * HOUR, Dst::None),
	zone("Australia/Brisbane", 10 * HOUR, Dst::None),
	zone("America/Sao_Paulo", -3 * HOUR, Dst::None),
	zone("America/New_York", -5 * HOUR, Dst::UnitedStates),
	zone("America/Chicago", -6 * HOUR, Dst::UnitedStates),
	zone("America/Denver", -7 * HOUR, Dst::UnitedStates),
	zone("America/Phoenix", -7 * HOUR, Dst::None),
	zone("America/Los_Angeles", -8 * HOUR, Dst::UnitedStates),
	zone("America/Anchorage", -9 * HOUR, Dst::UnitedStates),
	zone("Pacific/Honolulu", -10 * HOUR, Dst::None),
];

pub fn zone_named(name: &str) -> Result<&'static Zone, String> {
	ZONES.iter().find(|zone| zone.name == name).ok_or_else(|| {
		format!("unknown time zone {name:?}: not in the embedded zone table (src/time/calendar.rs)")
	})
}

pub fn is_leap(year: i64) -> bool {
	year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

pub fn days_in_month(year: i64, month: i64) -> i64 {
	match month {
		2 if is_leap(year) => 29,
		2 => 28,
		4 | 6 | 9 | 11 => 30,
		_ => 31,
	}
}

/// Days since 1970-01-01 (Howard Hinnant's days_from_civil)
pub fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
	let year = if month <= 2 { year - 1 } else { year };
	let era = year.div_euclid(400);
	let year_of_era = year - era * 400;
	let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
	let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
	era * 146_097 + day_of_era - 719_468
}

pub fn civil_from_days(days: i64) -> Date {
	let days = days + 719_468;
	let era = days.div_euclid(146_097);
	let day_of_era = days - era * 146_097;
	let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146_096) / 365;
	let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
	let shifted_month = (5 * day_of_year + 2) / 153;
	let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
	let month = if shifted_month < 10 { shifted_month + 3 } else { shifted_month - 9 };
	let year = year_of_era + era * 400 + if month <= 2 { 1 } else { 0 };
	Date { year, month, day }
}

/// ISO weekday of a day count: Monday is 1, Sunday is 7
fn weekday_of(days: i64) -> i64 {
	(days + 3).rem_euclid(7) + 1
}

impl Date {
	/// Invalid fields are errors, never rolled over: `date(2024,2,30)` is not March 1
	pub fn new(year: i64, month: i64, day: i64) -> Result<Date, String> {
		if !(1..=12).contains(&month) {
			return Err(format!("month out of range: {month} (months are 1 to 12)"));
		}
		if !(1..=days_in_month(year, month)).contains(&day) {
			return Err(format!("day out of range: {year:04}-{month:02}-{day:02}"));
		}
		Ok(Date { year, month, day })
	}

	pub fn days(self) -> i64 {
		days_from_civil(self.year, self.month, self.day)
	}

	pub fn weekday(self) -> i64 {
		weekday_of(self.days())
	}

	pub fn day_of_year(self) -> i64 {
		self.days() - days_from_civil(self.year, 1, 1) + 1
	}

	/// Months first, then days, like Temporal and java.time
	pub fn shift(self, months: i64, days: i64, overflow: Overflow) -> Result<Date, String> {
		let total = self.year * 12 + self.month - 1 + months;
		let (year, month) = (total.div_euclid(12), total.rem_euclid(12) + 1);
		let last = days_in_month(year, month);
		let day = match overflow {
			_ if self.day <= last => self.day,
			Overflow::Clamp => last,
			Overflow::Reject => {
				return Err(format!(
					"{year:04}-{month:02}-{:02} does not exist: use add(…, overflow: clamp) for {year:04}-{month:02}-{last:02}",
					self.day
				))
			}
		};
		Ok(civil_from_days(days_from_civil(year, month, day) + days))
	}

	/// Last Sunday of a month
	fn last_sunday(year: i64, month: i64) -> i64 {
		let last = days_from_civil(year, month, days_in_month(year, month));
		last - weekday_of(last) % 7
	}

	/// n-th Sunday of a month, counting from 1
	fn nth_sunday(year: i64, month: i64, n: i64) -> i64 {
		let first = days_from_civil(year, month, 1);
		first + (7 - weekday_of(first)) % 7 + 7 * (n - 1)
	}
}

impl Clock {
	pub fn new(hour: i64, minute: i64, second: i64, nanos: i64) -> Result<Clock, String> {
		if !(0..24).contains(&hour) {
			return Err(format!("hour out of range: {hour}"));
		}
		if !(0..60).contains(&minute) {
			return Err(format!("minute out of range: {minute}"));
		}
		if !(0..60).contains(&second) {
			return Err(format!("second out of range: {second}"));
		}
		Ok(Clock { hour, minute, second, nanos })
	}

	fn nanos_of_day(self) -> i128 {
		(self.hour * 3600 + self.minute * 60 + self.second) as i128 * SECOND + self.nanos as i128
	}

	fn from_nanos_of_day(nanos: i128) -> Clock {
		let seconds = (nanos / SECOND) as i64;
		Clock { hour: seconds / 3600, minute: seconds / 60 % 60, second: seconds % 60, nanos: (nanos % SECOND) as i64 }
	}
}

/// Local wall clock as nanoseconds since 1970-01-01T00:00 on the same wall
fn wall_nanos(date: Date, clock: Clock) -> i128 {
	date.days() as i128 * DAY + clock.nanos_of_day()
}

fn from_wall_nanos(nanos: i128) -> (Date, Clock) {
	let days = nanos.div_euclid(DAY) as i64;
	(civil_from_days(days), Clock::from_nanos_of_day(nanos.rem_euclid(DAY)))
}

pub fn instant_of(date: Date, clock: Clock, offset: i64) -> Instant {
	wall_nanos(date, clock) - offset as i128 * SECOND
}

pub fn local_at(instant: Instant, offset: i64) -> (Date, Clock) {
	from_wall_nanos(instant + offset as i128 * SECOND)
}

impl Zone {
	/// Seconds east of UTC in effect at an instant
	pub fn offset_at(&self, instant: Instant) -> i64 {
		let seconds = instant.div_euclid(SECOND) as i64;
		let year = civil_from_days(seconds.div_euclid(DAY_SECONDS)).year;
		let (start, end) = match self.dst {
			Dst::None => return self.standard,
			Dst::Europe => (
				Date::last_sunday(year, 3) * DAY_SECONDS + HOUR,
				Date::last_sunday(year, 10) * DAY_SECONDS + HOUR,
			),
			Dst::UnitedStates => (
				Date::nth_sunday(year, 3, 2) * DAY_SECONDS + 2 * HOUR - self.standard,
				Date::nth_sunday(year, 11, 1) * DAY_SECONDS + 2 * HOUR - self.standard - HOUR,
			),
		};
		if (start..end).contains(&seconds) {
			self.standard + HOUR
		} else {
			self.standard
		}
	}

	/// A wall time skipped by a transition is an error, never a silent shift; a repeated one takes the earlier instant
	pub fn resolve(&'static self, date: Date, clock: Clock) -> Result<Instant, String> {
		let offsets = [self.standard + HOUR, self.standard];
		let offsets = if self.dst == Dst::None { &offsets[1..] } else { &offsets[..] };
		offsets
			.iter()
			.map(|offset| (instant_of(date, clock, *offset), *offset))
			.filter(|(instant, offset)| self.offset_at(*instant) == *offset)
			.map(|(instant, _)| instant)
			.min()
			.ok_or_else(|| {
				format!(
					"{} does not exist in {}: skipped by a daylight saving transition",
					Time::Local(date, clock),
					self.name
				)
			})
	}
}

/// Length of an RFC 3339 / RFC 9557 literal at the start of `chars`, 0 if there is none.
/// Only the strict form counts (4-digit year, 2-digit month and day, no spaces): `2024 - 1 - 31` stays arithmetic.
pub fn literal_len(chars: &[char]) -> usize {
	let digits = |from: usize, count: usize| (from..from + count).all(|i| chars.get(i).is_some_and(char::is_ascii_digit));
	let is = |i: usize, c: char| chars.get(i) == Some(&c);
	if !(digits(0, 4) && is(4, '-') && digits(5, 2) && is(7, '-') && digits(8, 2)) {
		return 0;
	}
	let mut end = 10;
	if (is(end, 'T') || is(end, 't')) && digits(end + 1, 2) && is(end + 3, ':') && digits(end + 4, 2) {
		end += 6;
		if is(end, ':') && digits(end + 1, 2) {
			end += 3;
			if is(end, '.') && digits(end + 1, 1) {
				end += 1;
				while chars.get(end).is_some_and(char::is_ascii_digit) {
					end += 1;
				}
			}
		}
		if is(end, 'Z') || is(end, 'z') {
			end += 1;
		} else if (is(end, '+') || is(end, '-')) && digits(end + 1, 2) && is(end + 3, ':') && digits(end + 4, 2) {
			end += 6;
		}
		if is(end, '[') {
			let close = (end + 1..chars.len()).find(|&i| chars[i] == ']');
			if let Some(close) = close {
				let name = &chars[end + 1..close];
				if !name.is_empty() && name.iter().all(|c| c.is_ascii_alphanumeric() || "/_+-".contains(*c)) {
					end = close + 1;
				}
			}
		}
	}
	let at_boundary = chars.get(end).is_none_or(|c| !(c.is_alphanumeric() || *c == '_'));
	if at_boundary {
		end
	} else {
		0
	}
}

/// Validate a literal found by `literal_len`: an offset makes an instant, a bracketed zone a zoned time
pub fn parse_literal(text: &str) -> Result<Time, String> {
	let bytes = text.as_bytes();
	let number = |from: usize, to: usize| text.get(from..to).and_then(|digits| digits.parse::<i64>().ok()).unwrap_or(0);
	let date = Date::new(number(0, 4), number(5, 7), number(8, 10))?;
	if bytes.len() == 10 {
		return Ok(Time::Date(date));
	}
	let (hour, minute) = (number(11, 13), number(14, 16));
	let mut i = 16;
	let (mut second, mut nanos) = (0, 0);
	if bytes.get(i) == Some(&b':') {
		second = number(i + 1, i + 3);
		i += 3;
		if bytes.get(i) == Some(&b'.') {
			let start = i + 1;
			i = start;
			while bytes.get(i).is_some_and(u8::is_ascii_digit) {
				i += 1;
			}
			let fraction = &text[start..i.min(start + 9)];
			nanos = format!("{fraction:0<9}").parse().unwrap_or(0);
		}
	}
	let clock = Clock::new(hour, minute, second, nanos)?;
	let mut universal = false;
	let offset = match bytes.get(i) {
		Some(b'Z' | b'z') => {
			i += 1;
			universal = true;
			Some(0)
		}
		Some(sign @ (b'+' | b'-')) => {
			let (hours, minutes) = (number(i + 1, i + 3), number(i + 4, i + 6));
			if hours > 23 || minutes > 59 {
				return Err(format!("offset out of range in {text}"));
			}
			let seconds = hours * HOUR + minutes * 60;
			i += 6;
			Some(if *sign == b'-' { -seconds } else { seconds })
		}
		_ => None,
	};
	let zone = match bytes.get(i) {
		Some(b'[') => Some(zone_named(&text[i + 1..text.len() - 1])?),
		_ => None,
	};
	match (offset, zone) {
		(None, None) => Ok(Time::Local(date, clock)),
		(Some(offset), None) => Ok(Time::Instant(instant_of(date, clock, offset))),
		(None, Some(zone)) => Ok(Time::Zoned(zone.resolve(date, clock)?, zone)),
		(Some(offset), Some(zone)) => {
			let instant = instant_of(date, clock, offset);
			// RFC 9557: `Z[zone]` names only the instant, a numeric offset must agree with the zone
			if !universal && zone.offset_at(instant) != offset {
				return Err(format!("offset in {text} does not match {}", zone.name));
			}
			Ok(Time::Zoned(instant, zone))
		}
	}
}

/// Duration of one time unit word: `month`, `24 hours`
pub fn unit_named(word: &str) -> Option<Duration> {
	let exact = |seconds: i128| Duration { nanos: seconds * SECOND, ..Duration::default() };
	Some(match word {
		"year" | "years" => Duration { months: 12, ..Duration::default() },
		"month" | "months" => Duration { months: 1, ..Duration::default() },
		"week" | "weeks" => Duration { days: 7, ..Duration::default() },
		"day" | "days" => Duration { days: 1, ..Duration::default() },
		"hour" | "hours" => exact(3600),
		"minute" | "minutes" => exact(60),
		"second" | "seconds" => exact(1),
		_ => return None,
	})
}

/// A time unit word after a number (`1 month`): its duration and how many chars it spans including leading spaces
pub fn unit_after(chars: &[char]) -> Option<(Duration, usize)> {
	let start = chars.iter().take_while(|c| **c == ' ').count();
	let length = chars[start..].iter().take_while(|c| c.is_alphanumeric() || **c == '_').count();
	let word: String = chars[start..start + length].iter().collect();
	unit_named(&word).map(|unit| (unit, start + length))
}

impl Duration {
	pub fn times(self, factor: i64) -> Duration {
		Duration { months: self.months * factor, days: self.days * factor, nanos: self.nanos * factor as i128 }
	}

	pub fn plus(self, other: Duration) -> Duration {
		Duration { months: self.months + other.months, days: self.days + other.days, nanos: self.nanos + other.nanos }
	}

	fn is_exact(self) -> bool {
		self.months == 0 && self.days == 0
	}

	/// Fields of a duration: `(24 hours).hours` → 24
	pub fn field(self, name: &str) -> Result<i64, String> {
		let exact = |unit: i128| {
			if self.is_exact() {
				Ok((self.nanos / (unit * SECOND)) as i64)
			} else {
				Err(format!("{self} has calendar parts: {name} depends on the dates it is added to"))
			}
		};
		match name {
			"months" => Ok(self.months),
			"days" if self.months == 0 && self.nanos == 0 => Ok(self.days),
			"hours" => exact(3600),
			"minutes" => exact(60),
			"seconds" => exact(1),
			_ => Err(format!("duration has no field {name}")),
		}
	}
}

impl Time {
	/// Name of the kind, used in every error that refuses to mix kinds
	pub fn kind(&self) -> &'static str {
		match self {
			Time::Date(_) => "date",
			Time::Local(..) => "local time",
			Time::Instant(_) => "instant",
			Time::Zoned(..) => "zoned time",
		}
	}

	/// `t in "Europe/Berlin"`: an instant gets a wall clock, a local time gets an instant
	pub fn in_zone(self, zone: &'static Zone) -> Result<Time, String> {
		match self {
			Time::Instant(instant) | Time::Zoned(instant, _) => Ok(Time::Zoned(instant, zone)),
			Time::Local(date, clock) => Ok(Time::Zoned(zone.resolve(date, clock)?, zone)),
			Time::Date(date) => Err(format!("a date has no time of day: give {date} a clock before placing it in {}", zone.name)),
		}
	}

	fn calendar(self) -> Option<(Date, Option<Clock>)> {
		match self {
			Time::Date(date) => Some((date, None)),
			Time::Local(date, clock) => Some((date, Some(clock))),
			Time::Zoned(instant, zone) => {
				let (date, clock) = local_at(instant, zone.offset_at(instant));
				Some((date, Some(clock)))
			}
			Time::Instant(_) => None,
		}
	}

	/// Months are 1-based: `2024-02-29.month` → 2
	pub fn field(self, name: &str) -> Result<i64, String> {
		let kind = self.kind();
		let Some((date, clock)) = self.calendar() else {
			return match (self, name) {
				(Time::Instant(instant), "epoch_seconds") => Ok(instant.div_euclid(SECOND) as i64),
				_ => Err(format!("instant has no {name}: place it in a zone first, e.g. `now in \"Europe/Berlin\"`")),
			};
		};
		let clock_field = |pick: fn(Clock) -> i64| clock.map(pick).ok_or_else(|| format!("{kind} has no {name}"));
		match name {
			"year" => Ok(date.year),
			"month" => Ok(date.month),
			"day" => Ok(date.day),
			"weekday" => Ok(date.weekday()),
			"day_of_year" => Ok(date.day_of_year()),
			"hour" => clock_field(|clock| clock.hour),
			"minute" => clock_field(|clock| clock.minute),
			"second" => clock_field(|clock| clock.second),
			"offset" => match self {
				Time::Zoned(instant, zone) => Ok(zone.offset_at(instant)),
				_ => Err(format!("{kind} has no offset")),
			},
			_ => Err(format!("{kind} has no field {name}")),
		}
	}

	/// `2024-01-31 + 1 month` is an error unless the overflow is spelled out
	pub fn add(self, duration: Duration, overflow: Overflow) -> Result<Time, String> {
		let Duration { months, days, nanos } = duration;
		match self {
			Time::Date(_) if nanos != 0 => Err(format!("a date has no time of day: add {duration} to a local time")),
			Time::Date(date) => Ok(Time::Date(date.shift(months, days, overflow)?)),
			Time::Local(date, clock) => {
				let date = date.shift(months, days, overflow)?;
				let (date, clock) = from_wall_nanos(wall_nanos(date, clock) + nanos);
				Ok(Time::Local(date, clock))
			}
			Time::Instant(_) if !duration.is_exact() => Err(format!(
				"an instant has no calendar: place it in a zone before adding {duration}"
			)),
			Time::Instant(instant) => Ok(Time::Instant(instant + nanos)),
			Time::Zoned(instant, zone) => {
				let instant = if duration.is_exact() {
					instant
				} else {
					let (date, clock) = local_at(instant, zone.offset_at(instant));
					zone.resolve(date.shift(months, days, overflow)?, clock)?
				};
				Ok(Time::Zoned(instant + nanos, zone))
			}
		}
	}

	fn position(self) -> i128 {
		match self {
			Time::Date(date) => date.days() as i128,
			Time::Local(date, clock) => wall_nanos(date, clock),
			Time::Instant(instant) | Time::Zoned(instant, _) => instant,
		}
	}

	fn same_kind(self, other: Time, verb: &str) -> Result<(), String> {
		if self.kind() == other.kind() {
			Ok(())
		} else {
			Err(format!(
				"cannot {verb} a {} and a {}: no implicit conversion between kinds of time",
				self.kind(),
				other.kind()
			))
		}
	}

	/// Only the same kind compares: a date is not midnight, a local time is not in any zone
	pub fn compare(self, other: Time) -> Result<Ordering, String> {
		self.same_kind(other, "compare")?;
		Ok(self.position().cmp(&other.position()))
	}

	/// `date - date` → days; other kinds give an exact duration
	pub fn since(self, other: Time) -> Result<Result<i64, Duration>, String> {
		self.same_kind(other, "subtract")?;
		let difference = self.position() - other.position();
		Ok(match self {
			Time::Date(_) => Ok(difference as i64),
			_ => Err(Duration { nanos: difference, ..Duration::default() }),
		})
	}
}

fn write_clock(f: &mut fmt::Formatter<'_>, clock: Clock) -> fmt::Result {
	write!(f, "T{:02}:{:02}", clock.hour, clock.minute)?;
	if clock.second != 0 || clock.nanos != 0 {
		write!(f, ":{:02}", clock.second)?;
	}
	if clock.nanos != 0 {
		let fraction = format!("{:09}", clock.nanos);
		write!(f, ".{}", fraction.trim_end_matches('0'))?;
	}
	Ok(())
}

impl fmt::Display for Date {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
	}
}

impl fmt::Display for Time {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match *self {
			Time::Date(date) => write!(f, "{date}"),
			Time::Local(date, clock) => {
				write!(f, "{date}")?;
				write_clock(f, clock)
			}
			Time::Instant(instant) => {
				let (date, clock) = local_at(instant, 0);
				write!(f, "{date}")?;
				write_clock(f, clock)?;
				write!(f, "Z")
			}
			Time::Zoned(instant, zone) => {
				let offset = zone.offset_at(instant);
				let (date, clock) = local_at(instant, offset);
				write!(f, "{date}")?;
				write_clock(f, clock)?;
				let sign = if offset < 0 { '-' } else { '+' };
				write!(f, "{sign}{:02}:{:02}[{}]", offset.abs() / 3600, offset.abs() / 60 % 60, zone.name)
			}
		}
	}
}

impl fmt::Display for Duration {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut parts = vec![];
		if self.months != 0 {
			parts.push(format!("{} months", self.months));
		}
		if self.days != 0 {
			parts.push(format!("{} days", self.days));
		}
		if self.nanos != 0 || parts.is_empty() {
			let seconds = self.nanos as f64 / SECOND as f64;
			parts.push(format!("{seconds} seconds"));
		}
		write!(f, "{}", parts.join(" "))
	}
}

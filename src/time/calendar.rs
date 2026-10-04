//! Calendar arithmetic without host dependencies: proleptic Gregorian dates, UTC instants and an embedded zone table.
//! Four distinct kinds of time never convert implicitly (Footguns.md → Dates and time zones).

use std::cmp::Ordering;
use std::fmt;

const SECOND: i128 = 1_000_000_000;
const DAY_SECONDS: i64 = 86_400;
const HOUR: i64 = 3600;
const MINUTE: i64 = 60;
/// Howard Hinnant's civil calendar arithmetic: a 400-year era has 146 097 days; day 0 of the shifted calendar (which
/// starts in March of year 0) is 719 468 days before 1970-01-01
const DAYS_PER_ERA: i64 = 146_097;
const YEARS_PER_ERA: i64 = 400;
const EPOCH_SHIFT: i64 = 719_468;
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
	/// `2024-01-31T10:00+01:00[Europe/Berlin]`: a wall time in a zone, the instant is derived
	Zoned(Zoned),
}

/// The value of a zoned time is its wall time and zone. The offset is the one chosen when it was resolved,
/// under the rules version it was resolved with; the instant is derived from them. Future rule changes are
/// politics: a program cannot know them, it can only notice when rules it was resolved with have changed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zoned {
	pub date: Date,
	pub clock: Clock,
	pub offset: i64, // seconds east of UTC, one of the zone's offsets at this wall time
	pub zone: &'static Zone,
	pub rules: &'static TzRules,
}

/// Which instant a wall time means when a transition skips it or repeats it (JavaScript Temporal's `disambiguation`)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Disambiguation {
	/// The default: a skipped or repeated wall time is an error listing both instants
	Reject,
	/// The first of two repeated instants; in a gap, the wall time shifted back by the gap
	Earlier,
	/// The second of two repeated instants; in a gap, the wall time shifted forward by the gap
	Later,
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

/// A version of the time zone rules: times after `published` are predictions that a later version may revise
#[derive(Debug, PartialEq)]
pub struct TzRules {
	pub version: &'static str,
	pub published: Date,
	pub zones: &'static [Zone],
}

/// The rules compiled into warp
pub static BUILTIN_RULES: TzRules = TzRules {
	version: "warp-2026a",
	published: Date { year: 2026, month: 9, day: 28 },
	zones: ZONES,
};

thread_local! {
	static CURRENT_RULES: std::cell::Cell<&'static TzRules> = const { std::cell::Cell::new(&BUILTIN_RULES) };
}

/// The rules times are resolved with now
pub fn rules() -> &'static TzRules {
	CURRENT_RULES.with(|current| current.get())
}

/// Resolve times with other rules while `body` runs, e.g. to simulate a rule change in a test
pub fn with_rules<R>(rules: &'static TzRules, body: impl FnOnce() -> R) -> R {
	struct Restore(&'static TzRules);
	impl Drop for Restore {
		fn drop(&mut self) {
			CURRENT_RULES.with(|current| current.set(self.0));
		}
	}
	let _restore = Restore(CURRENT_RULES.with(|current| current.replace(rules)));
	body()
}

pub const fn zone(name: &'static str, standard: i64, dst: Dst) -> Zone {
	Zone { name, standard, dst }
}


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
	let rules = rules();
	rules.zones.iter().find(|zone| zone.name == name).ok_or_else(|| {
		format!("unknown time zone {name:?}: not in the time zone rules {} (src/time/calendar.rs)", rules.version)
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
	let era = year.div_euclid(YEARS_PER_ERA);
	let year_of_era = year - era * YEARS_PER_ERA;
	let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
	let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
	era * DAYS_PER_ERA + day_of_era - EPOCH_SHIFT
}

pub fn civil_from_days(days: i64) -> Date {
	let days = days + EPOCH_SHIFT;
	let era = days.div_euclid(DAYS_PER_ERA);
	let day_of_era = days - era * DAYS_PER_ERA;
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
		(self.hour * HOUR + self.minute * MINUTE + self.second) as i128 * SECOND + self.nanos as i128
	}

	fn from_nanos_of_day(nanos: i128) -> Clock {
		let seconds = (nanos / SECOND) as i64;
		Clock { hour: seconds / HOUR, minute: seconds / MINUTE % MINUTE, second: seconds % MINUTE, nanos: (nanos % SECOND) as i64 }
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

	/// A wall time skipped or repeated by a transition is an error unless the choice is spelled out, never a silent shift
	pub fn resolve(&'static self, date: Date, clock: Clock, choice: Disambiguation) -> Result<Zoned, String> {
		let offsets = [self.standard + HOUR, self.standard];
		let offsets = if self.dst == Dst::None { &offsets[1..] } else { &offsets[..] };
		let valid: Vec<i64> = offsets.iter().copied().filter(|offset| self.offset_at(instant_of(date, clock, *offset)) == *offset).collect();
		if let [offset] = valid[..] {
			return Ok(Zoned { date, clock, offset, zone: self, rules: rules() });
		}
		// Both offsets give the two candidates: the repeated instants, or the wall time shifted across the gap
		let instants = offsets.iter().map(|offset| instant_of(date, clock, *offset));
		let earlier = Zoned::at(instants.clone().min().unwrap_or_default(), self);
		let later = Zoned::at(instants.max().unwrap_or_default(), self);
		let wall = Time::Local(date, clock);
		match choice {
			Disambiguation::Earlier => Ok(earlier),
			Disambiguation::Later => Ok(later),
			Disambiguation::Reject if valid.is_empty() => Err(format!(
				"{wall} does not exist in {}: skipped by a daylight saving transition. Choose disambiguation: earlier → {} or later → {}",
				self.name,
				earlier.plain(),
				later.plain()
			)),
			Disambiguation::Reject => Err(format!(
				"{wall} is ambiguous in {}: it occurs twice, the clocks are set back by a daylight saving transition. Write {} (earlier) or {} (later), or choose disambiguation: earlier or later",
				self.name,
				earlier.plain(),
				later.plain()
			)),
		}
	}
}

impl Zoned {
	/// The wall time of an instant in a zone, under the current rules
	pub fn at(instant: Instant, zone: &'static Zone) -> Zoned {
		let offset = zone.offset_at(instant);
		let (date, clock) = local_at(instant, offset);
		Zoned { date, clock, offset, zone, rules: rules() }
	}

	pub fn instant(self) -> Instant {
		instant_of(self.date, self.clock, self.offset)
	}

	/// Later than the publication of its rules: the offset is a prediction
	pub fn is_prediction(self) -> bool {
		self.instant() >= self.rules.published.days() as i128 * DAY
	}

	/// RFC 9557 form without the rules version: `2030-10-27T02:30+01:00[Europe/Berlin]`
	pub fn plain(self) -> String {
		let mut text = Time::Local(self.date, self.clock).to_string();
		let (sign, offset) = if self.offset < 0 { ('-', -self.offset) } else { ('+', self.offset) };
		text += &format!("{sign}{:02}:{:02}[{}]", offset / HOUR, offset / MINUTE % MINUTE, self.zone.name);
		text
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
		// RFC 9557 suffixes: `[Europe/Berlin]`, then `[key=value]` like `[_tzdata=warp-2026a]`
		while is(end, '[') {
			let Some(close) = (end + 1..chars.len()).find(|&i| chars[i] == ']') else { break };
			let name = &chars[end + 1..close];
			if name.is_empty() || !name.iter().all(|c| c.is_ascii_alphanumeric() || "/_+-=.!".contains(*c)) {
				break;
			}
			end = close + 1;
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
	let (mut zone, mut resolved_with) = (None, None);
	for suffix in text[i..].split_terminator(']') {
		let suffix = suffix.strip_prefix('[').unwrap_or(suffix);
		match suffix.split_once('=') {
			Some(("_tzdata", version)) => resolved_with = Some(version),
			Some(_) => return Err(format!("unsupported suffix [{suffix}] in {text}: only [_tzdata=version] is known")),
			None if zone.is_none() => zone = Some(zone_named(suffix)?),
			None => return Err(format!("{text} names two zones")),
		}
	}
	if resolved_with.is_some() && zone.is_none() {
		return Err(format!("{text}: only a zoned time records the time zone rules it was resolved with"));
	}
	match (offset, zone) {
		(None, None) => Ok(Time::Local(date, clock)),
		(Some(offset), None) => Ok(Time::Instant(instant_of(date, clock, offset))),
		// The wall time is the value: resolving it with newer rules is what the writer meant
		(None, Some(zone)) => Ok(Time::Zoned(zone.resolve(date, clock, Disambiguation::Reject)?)),
		// RFC 9557: `Z[zone]` names only the instant
		(Some(_), Some(zone)) if universal => Ok(Time::Zoned(Zoned::at(instant_of(date, clock, 0), zone))),
		(Some(offset), Some(zone)) => {
			let instant = instant_of(date, clock, offset);
			if zone.offset_at(instant) == offset {
				return Ok(Time::Zoned(Zoned { date, clock, offset, zone, rules: rules() }));
			}
			// The numeric offset disagrees with the zone: never pick the wall time or the instant silently
			let keep_wall = match zone.resolve(date, clock, Disambiguation::Reject) {
				Ok(zoned) => zoned.plain(),
				Err(message) => format!("none ({message})"),
			};
			let keep_instant = Zoned::at(instant, zone).plain();
			let current = rules().version;
			let reason = match resolved_with {
				Some(version) if version != current => format!(
					"was resolved with time zone rules {version}, under {current} the offset of {} at {} is different",
					zone.name,
					Time::Local(date, clock)
				),
				_ => format!("offset does not match {}", zone.name),
			};
			Err(format!("{text} {reason}: keep the wall time → {keep_wall}, or keep the instant → {keep_instant}"))
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
		"hour" | "hours" => exact(HOUR as i128),
		"minute" | "minutes" => exact(MINUTE as i128),
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

	/// `1 day == 24 hours` depends on the day: equal fields are equal, one differing field is unequal, more is an error
	pub fn equals(self, other: Duration) -> Result<bool, String> {
		let differing = [self.months != other.months, self.days != other.days, self.nanos != other.nanos];
		match differing.iter().filter(|differs| **differs).count() {
			0 => Ok(true),
			1 => Ok(false),
			_ => Err(format!(
				"{self} and {other} are equal on some dates and not on others (a day is 23 to 25 hours across daylight saving, a month 28 to 31 days): compare the times they lead to"
			)),
		}
	}

	pub fn is_exact(self) -> bool {
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
			"hours" => exact(HOUR as i128),
			"minutes" => exact(MINUTE as i128),
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
			Time::Instant(instant) => Ok(Time::Zoned(Zoned::at(instant, zone))),
			Time::Zoned(zoned) => Ok(Time::Zoned(Zoned::at(zoned.instant(), zone))),
			Time::Local(date, clock) => Ok(Time::Zoned(zone.resolve(date, clock, Disambiguation::Reject)?)),
			Time::Date(date) => Err(format!("a date has no time of day: give {date} a clock before placing it in {}", zone.name)),
		}
	}

	fn calendar(self) -> Option<(Date, Option<Clock>)> {
		match self {
			Time::Date(date) => Some((date, None)),
			Time::Local(date, clock) => Some((date, Some(clock))),
			Time::Zoned(zoned) => Some((zoned.date, Some(zoned.clock))),
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
				Time::Zoned(zoned) => Ok(zoned.offset),
				_ => Err(format!("{kind} has no offset")),
			},
			_ => Err(format!("{kind} has no field {name}")),
		}
	}

	/// `2024-01-31 + 1 month` is an error unless the overflow is spelled out.
	/// On a zoned time calendar units keep the wall time (`+ 1 day`), exact units keep the elapsed time (`+ 24 hours`)
	pub fn add(self, duration: Duration, overflow: Overflow, choice: Disambiguation) -> Result<Time, String> {
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
			Time::Zoned(zoned) => {
				let zoned = if duration.is_exact() {
					zoned
				} else {
					zoned.zone.resolve(zoned.date.shift(months, days, overflow)?, zoned.clock, choice)?
				};
				Ok(Time::Zoned(if nanos == 0 { zoned } else { Zoned::at(zoned.instant() + nanos, zoned.zone) }))
			}
		}
	}

	fn position(self) -> i128 {
		match self {
			Time::Date(date) => date.days() as i128,
			Time::Local(date, clock) => wall_nanos(date, clock),
			Time::Instant(instant) => instant,
			Time::Zoned(zoned) => zoned.instant(),
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
			// A prediction records the rules it was resolved with, so a reader with other rules can tell
			Time::Zoned(zoned) if zoned.is_prediction() => write!(f, "{}[_tzdata={}]", zoned.plain(), zoned.rules.version),
			Time::Zoned(zoned) => write!(f, "{}", zoned.plain()),
		}
	}
}

impl fmt::Display for Duration {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let count = |amount: i128, unit: &str| if amount == 1 { format!("1 {unit}") } else { format!("{amount} {unit}s") };
		let mut parts = vec![];
		if self.months != 0 {
			parts.push(count(self.months as i128, "month"));
		}
		if self.days != 0 {
			parts.push(count(self.days as i128, "day"));
		}
		// whole seconds read as hours, minutes and seconds: `3 hours`, not `10800 seconds`
		if self.nanos % SECOND == 0 && self.nanos != 0 {
			let (seconds, hour, minute) = (self.nanos / SECOND, HOUR as i128, MINUTE as i128);
			for (amount, unit) in [(seconds / hour, "hour"), (seconds % hour / minute, "minute"), (seconds % minute, "second")] {
				if amount != 0 {
					parts.push(count(amount, unit));
				}
			}
		} else if self.nanos != 0 || parts.is_empty() {
			let seconds = self.nanos as f64 / SECOND as f64;
			parts.push(format!("{seconds} seconds"));
		}
		write!(f, "{}", parts.join(" "))
	}
}

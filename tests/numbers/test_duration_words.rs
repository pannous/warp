// card duration-warp: runtime.warp's duration words show and order the three amounts of a duration as calendar.rs
// Duration does. The section is compiled here as an ordinary program, its intrinsics by stand-ins of plain warp and
// its failure by 0 (`fail` is a soft keyword)
use warp::time::calendar::Duration;
use warp::*;

const RUNTIME_WORDS: &str = "lib/compiler/runtime.warp";
const DURATIONS_SECTION: &str = "## durations";
/// The section ends where the next starts, after an empty line: its own header goes on over several `##` lines
const NEXT_SECTION: &str = "\n\n## ";
const SECOND: i128 = 1_000_000_000;
/// The intrinsics and runtime words the section calls, in the language itself; divisors are positive there
const STAND_INS: &str = r#"
int_text(n: int): text := "" + n
int_quotient(a: int, b: int): int := if a < 0 and a % b != 0 then a /! b + 1 else a /! b
int_remainder(a: int, b: int): int := a - int_quotient(a, b) * b
floor_quotient(a: int, b: int): int := a /! b
byte_count(t: text): int := #t
zeros(count: int): text := {
	out = ""
	while #out < count { out = out + "0" }
	out
}
power(base: int, exponent: int): int := {
	out = 1
	left = exponent
	while left > 0 {
		out = out * base
		left -= 1
	}
	out
}
marked_kind(n: any): int := 0
unboxed_int(n: any): int := 0
"#;

fn duration_words() -> String {
	let source = std::fs::read_to_string(RUNTIME_WORDS).unwrap();
	let section = source.split_once(DURATIONS_SECTION).expect("runtime.warp has a durations section").1;
	let section = section.split_once(NEXT_SECTION).map_or(section, |(words, _)| words);
	format!("{STAND_INS}\n# {}", section.replace("fail()", "0"))
}

fn duration_word(call: &str) -> String {
	wasm_emitter::eval(&format!("{}\n{call}", duration_words())).serialize()
}

#[test]
fn duration_text_reads_as_calendar_shows_a_duration() {
	let durations = [(0, 0, 0), (1, 2, 3 * 3600 * SECOND), (0, 0, 5400 * SECOND), (0, 1, 0), (-2, 0, 0)];
	let fractions = [(0, 0, SECOND * 3 / 2), (0, 0, -SECOND * 3 / 2), (0, 0, 1), (1, 0, SECOND / 4)];
	for (months, days, nanos) in durations.into_iter().chain(fractions) {
		let shown = Duration { months, days, nanos }.to_string();
		let text = duration_word(&format!("duration_parts_text({months}, {days}, {nanos})"));
		assert_eq!(text.trim_matches('"'), shown, "{months} months {days} days {nanos} nanoseconds");
	}
}

#[test]
fn duration_order_holds_on_every_date() {
	let day = 86400 * SECOND;
	for ((months, days, nanos), order) in
		[((0, 1, -2 * 3600 * SECOND), 1), ((0, 0, -5), -1), ((0, 0, 0), 0), ((1, -27, 0), 1), ((-1, 27, 0), -1)]
	{
		let call = format!("duration_parts_order({months}, {days}, {nanos})");
		assert_eq!(duration_word(&call), order.to_string(), "{call}");
	}
	assert_eq!(duration_word(&format!("duration_parts_order(0, 2, {})", -day)), "1");
}

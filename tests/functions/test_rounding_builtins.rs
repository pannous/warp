// floor, ceil, round and √ are wasm instructions giving exact Ints (√ a float), never libm calls, also under `use m`
// or `import floor from 'm'` (user 2026-10-10: "floor(v) should be int, upgradable to float"). libm's f64 floor made
// lib/draw.warp's cell fail ("floor is a float where an exact Int is expected") when draw was a local file
use crate::is;

#[test]
fn rounding_words_stay_ints_when_libm_is_used() {
	is!("use m; floor(3.7)", 3);
	is!("use m; floor(3.7) + 0.5", 3.5);
	is!("import floor from 'm'; floor(-2.3)", -3);
	is!("use math\ncell(v) := floor(v) as int\nlabel(xs) := cell(xs#3 / 255)\ncell(3)", 3);
	is!("use m; ceil(3.2) * 2", 8);
	is!("use m; cbrt(27.0)", 3.0);
}

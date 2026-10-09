// card quantity-tolerance: a run-time quantity whose amount is a ± value compares, reads its bounds, sums and fills
// unit fields like a ± number (notes/plus_minus.md)
use crate::common::fails_with;

const ROPE: &str = "rope = 5 m ± 1 cm\n";

fn shown(code: &str) -> String {
	warp::wasm_emitter::eval(code).serialize().trim_matches('"').to_string()
}

#[test]
fn a_quantity_with_tolerance_compares_certainly_or_possibly() {
	assert_eq!(shown(&format!("{ROPE}rope certainly > 4 m")), "yes");
	assert_eq!(shown(&format!("{ROPE}rope certainly > 4.995 m")), "no");
	assert_eq!(shown(&format!("{ROPE}rope possibly > 5.005 m")), "yes");
	assert_eq!(shown(&format!("{ROPE}rope certainly < 600 cm")), "yes");
}

#[test]
fn a_quantity_with_tolerance_has_its_bounds() {
	assert_eq!(shown(&format!("{ROPE}str(rope.low)")), "4.99m");
	assert_eq!(shown(&format!("{ROPE}str(rope.high)")), "5.01m");
	assert_eq!(shown(&format!("{ROPE}str(rope.value)")), "5m");
}

#[test]
fn quantities_with_tolerance_sum() {
	assert_eq!(shown("str(sum([5 m ± 1 cm, 3 m ± 2 cm]))"), "8.000 ± 0.030m");
}

#[test]
fn a_unit_field_takes_a_quantity_with_tolerance() {
	assert_eq!(shown("class Rope{length: m}\nr = Rope(5 m ± 1 cm)\nstr(r.length)"), "5.000 ± 0.010m");
	assert_eq!(shown("class Rope{length: m}\nr = Rope{length: 5 m ± 1 cm}\nstr(r.length * 2)"), "10.000 ± 0.020m");
	fails_with("class Rope{length: m}\nRope(5 kg ± 1 g)", "Dimension");
}

#[test]
fn a_field_type_with_tolerance_gives_its_values_the_tolerance() {
	const PART: &str = "class Part{length: m ± 1 mm}\n";
	assert_eq!(shown(&format!("{PART}p = Part(5 m)\nstr(p.length)")), "5.0000 ± 0.0010m");
	assert_eq!(shown(&format!("{PART}p = Part{{length: 5 m}}\nstr(p.length)")), "5.0000 ± 0.0010m");
	assert_eq!(shown(&format!("{PART}p = Part(5 m ± 1 cm)\nstr(p.length)")), "5.000 ± 0.010m");
}

#[test]
fn a_gaussian_quantity_shows_sigma_apart_from_its_unit() {
	assert_eq!(shown("x = 12 m ± 0.6σ; str(x)"), "12.00 ± 0.60σ m");
}

// a final ± value of a unit field shows its unit, in the field's unit (card quantity-final)
#[test]
fn a_final_unit_field_with_tolerance_shows_its_unit() {
	assert_eq!(shown("class Rope{length: m}\nr = Rope(5 m ± 1 cm)\nr.length"), "5.000 ± 0.010m");
	assert_eq!(shown("class Rope{length: cm}\nr = Rope(5 m ± 1 cm)\nr.length"), "500.0 ± 1.0cm");
	assert_eq!(shown("class Rope{length: m}\nr = Rope(5 m ± 1 cm)\nr"), "Rope{length:5.000 ± 0.010m}");
}

// the sum of run-time quantities is a quantity: its text, and a variable holding it computes on (card quantity-sum)
#[test]
fn a_sum_of_quantities_is_a_quantity() {
	assert_eq!(shown("use units\nstr(sum([quantity(\"5 m\"), quantity(\"3 m\")]))"), "8m");
	assert_eq!(shown("use units\nsum([quantity(\"5 m\"), quantity(\"3 m\")])"), "8m");
	assert_eq!(shown("use units\ns = sum([quantity(\"5 m\"), quantity(\"3 m\")])\ns * 2"), "16m");
}

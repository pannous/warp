// card list-literal, decision litre-suffix: `l` and `ha` are units; a litre glued to its number (`2L`) needs `use units`,
// since C spells its long double that way
use crate::common::fails_with;
use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn litre_and_hectare_are_units() {
	assert_eq!(eval("2 l in cm³").serialize(), "2000cm³");
	assert_eq!(eval("2 L").serialize(), "2dm³");
	is!("3 ha is area", true);
	is!("l = 2; 3 l", 6);
}

#[test]
fn a_glued_litre_needs_use_units() {
	fails_with("2L", "L is the litre unit here; add `use units`");
	fails_with("3l", "l is the litre unit here");
	assert_eq!(eval("use units\n2L in cm³").serialize(), "2000cm³");
	assert_eq!(eval("use units; 2L in cm³").serialize(), "2000cm³");
	is!("use units\n2l is volume", true);
	is!("0.1f + 0.2f == 0.3", false);
}

// a module alias the program binds wins over a unit of the same name (batch 11: `use list as l`)
#[test]
fn a_module_alias_wins_over_a_unit_name() {
	is!("use list as l\ncount(l.unique([1 1 2]))", 2);
	is!("use list as L\nL.unique([1 1 2]) == [1 2]", true);
	is!("use math as ha\nha.floor(4.5)", 4);
	is!("use math as N\nN.floor(4.5)", 4);
}

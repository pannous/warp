// card type-word: unit, nil and ø are aliases of the type name empty (the canonical word type(ø) prints, warp-e9)
use crate::is;

#[test]
fn unit_nil_and_ø_name_the_empty_type() {
	is!("x = ø; x is unit", true);
	is!("x = 3; x is unit", false);
	is!("f(x:any) := x is unit; f(ø)", true);
	is!("f(x:any) := x is unit; f(3)", false);
	is!("is_type(ø, \"empty\")", true);
	is!("is_type(ø, \"unit\")", true);
	is!("is_type(ø, \"nil\")", true);
	is!("f(x:unit) := 7; f(ø)", 7);
	is!("f(x:empty) := 7; f(ø)", 7);
	is!("f(x:nil) := 7; f(ø)", 7);
}

#[test]
fn a_variable_declared_empty_holds_ø_not_zero() {
	is!("x: ø = ø; x is unit", true);
	is!("x: ø = ø; x == 0", false);
}

#[test]
fn a_type_value_compares_with_any_word_of_its_type() {
	is!("type(ø) == empty", true);
	is!("type(ø) == Empty", true);
	is!("type(ø) == unit", true);
	is!("x = ø; type(x) == nil", true);
	is!("type(3) == empty", false);
	is!("type(\"ab\") == string", true);
	is!("x = 3; x == empty", false);
}

// card equality-operand: a type word compares as its type on either side of == and !=, not only on the right
#[test]
fn a_type_word_compares_on_either_side() {
	is!("exact == real", true);
	is!("integer == int", true);
	is!("string == type(\"ab\")", true);
	is!("int != integer", false);
	is!("type(\"ab\") != string", false);
	is!("type(\"ab\") != int", true);
	is!("unit == type(ø)", true);
	// a variable of a type word's name stays the variable
	is!("string = \"s\"; string == \"s\"", true);
	is!("string = \"s\"; \"s\" == string", true);
	is!("n = 0; for char in \"ab\" { if char == \"a\" { n += 1 } }; n", 1);
}

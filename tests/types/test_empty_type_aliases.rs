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

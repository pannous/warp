// P61 (user, 2026-10-05): `be` defines (wiki/be.md, an alias of `:=`), the typed form too; `is` always compares
// (the teaching error for `x is …` with an undefined x is warp-2d's branch p61-is-teaches-be)
use warp::is;

#[test]
fn test_be_defines_with_a_type() {
	is!("x be number 9; x+1", 10);
	is!("x be 100 times [0]; x.length", 100);
}

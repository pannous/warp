// `double 2`, `long 2`: every builtin type name works as a constructor word, not only int and float
use crate::is;

#[test]
fn test_type_constructor_words() {
	is!("double 2", 2);
	is!("long 2", 2);
	is!("float 2", 2);
	is!("int 2", 2);
}

#[test]
fn test_type_first_parameter() {
	is!("grow(double z):=z*2;grow 5", 10);
	is!("grow(int z):=z*2;grow 5", 10);
	is!("half(float x):=x/2;half 3", 1.5);
}

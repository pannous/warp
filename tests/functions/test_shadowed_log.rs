// A program's own function shadows the library word of its name (card user-function): `log(a, b)` is the logarithm
// to a base only when the program defines no log of its own
use crate::is;

#[test]
fn a_user_log_of_two_arguments_is_called() {
	is!("log(value, old) := value - old; log(7, 2)", 5);
}

#[test]
fn the_library_log_to_a_base_stays() {
	is!("log(8, 2)", 3.0);
}

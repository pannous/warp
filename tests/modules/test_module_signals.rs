// card module-signal-writes: a used module's main-level variables are the program's shared state, and a listener of the
// program sees the writes of the module's functions (modules::resolve runs before the signal passes)
use crate::is;

#[test]
fn a_listener_sees_the_writes_of_a_used_module() {
	is!("use tests/fixtures/settings_store; seen = 0; on change theme { seen += 1 }; toggle(); toggle(); seen", 2);
	is!("use tests/fixtures/settings_store; on change theme { print value }; toggle(); theme", "light");
}

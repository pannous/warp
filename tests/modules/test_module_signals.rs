// card module-signal-writes: a used module's main-level variables are the program's shared state, and a listener of the
// program sees the writes of the module's functions (modules::resolve runs before the signal passes)
use crate::is;

#[test]
fn a_listener_sees_the_writes_of_a_used_module() {
	is!("use tests/fixtures/settings_store; seen = 0; on change theme { seen += 1 }; toggle(); toggle(); seen", 2);
	is!("use tests/fixtures/settings_store; on change theme { print value }; toggle(); theme", "light");
}

// card web-stores: `stored` in a used module is the program's persisted signal, kept in the program's store
#[test]
fn a_stored_value_of_a_used_module_is_kept_by_the_program() {
	is!("use tests/fixtures/stored_settings; seen = 0; on change module_theme { seen += 1 }; toggle_theme(); seen", 1);
	is!("use tests/fixtures/stored_settings; module_theme", "light");
}

// An element's handler, written (`button{ on click {…} }`) or made by `bind:`, goes with the markup it shows: no
// "a native run never raises it" warning, which named the handler's internal name (input·1). A handler of the
// program's own (`on submit {…}`) still draws it

fn warns_never_raised(code: &str) -> bool {
	warp::errors::diagnostic::take_warnings();
	let lowered = warp::pipeline::lower(code);
	assert!(lowered.is_ok(), "{lowered:?}");
	warp::errors::diagnostic::take_warnings().iter().any(|warning| warning.message.contains("never raises it"))
}

#[test]
#[cfg(feature = "native")] // the warning is the native run's
fn an_element_s_handler_draws_no_native_warning() {
	assert!(!warns_never_raised("x = \"\"\ninput{ bind: x }"));
	assert!(!warns_never_raised("x = 0\nbutton{ on click { x = 1 } \"b\" }"));
	assert!(!warns_never_raised("fruit = \"\"\nselect{ bind: fruit; [\"apple\" \"pear\"] }"));
	assert!(warns_never_raised("x = 0\non submit { x = 1 }\nx"));
}

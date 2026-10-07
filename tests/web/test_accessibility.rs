//! Accessible markup (card web-a11y, src/accessibility.rs): welcoming warnings for markup a screen reader or keyboard
//! user cannot use, each at the element as written with a fix
use warp::diagnostic::take_warnings;
use warp::wasm_emitter::eval;

fn warnings_of(code: &str) -> Vec<String> {
	take_warnings();
	eval(code);
	take_warnings().into_iter().map(|warning| warning.message).collect()
}

#[test]
fn unusable_markup_is_warned_about() {
	let warned = |code: &str, expected: &str| assert_eq!(warnings_of(code), vec![expected.to_string()], "{code}");
	warned("div{ img{ src: \"apple.png\" } }", "img without alt: a screen reader reads its file name");
	warned("div{ input{ placeholder: \"name\" } }", "input labeled only by its placeholder, which vanishes on typing");
	warned("div{ textarea{} }", "textarea without a label: a screen reader cannot say what it asks for");
	warned("div{ button{ img{ src: \"x.png\" alt: \"\" } } }", "button without text: a screen reader announces just \"button\"");
	warned("div{ a{ \"home\" } }", "a without href: not reachable by keyboard");
	warned("div{ a{ href: \"/\" } }", "a without text: a screen reader reads its address");
	warned("div{ h1: \"Shop\" h3: \"Fruit\" }", "h3 after h1 skips a heading level");
	warned("div{ p{ id: \"x\" \"a\" } p{ id: \"x\" \"b\" } }", "id \"x\" is used twice: labels and links find only the first");
	warned("html{ body{ p: \"x\" } }", "html without lang: screen readers guess the language");
}

#[test]
fn accessible_markup_draws_no_warning() {
	let fine = "html{ lang: \"en\" body{\n\th1: \"Shop\"\n\th2: \"Fruit\"\n\timg{ src: \"line.png\" alt: \"\" }\n\tinput{ id: \"email\" }\n\tlabel{ for: \"email\" \"Email\" }\n\tlabel{ \"Age\" input{ type: \"number\" } }\n\tinput{ type: \"hidden\" name: \"token\" }\n\tinput{ aria-label: \"search\" }\n\tbutton{ \"Save\" }\n\tbutton{ aria-label: \"close\" }\n\ta{ href: \"/\" \"home\" }\n} }";
	assert_eq!(warnings_of(fine), Vec::<String>::new());
	assert_eq!(warnings_of("count = 0\ndiv{ button{ \"Add\" } p{ count } }"), Vec::<String>::new());
}

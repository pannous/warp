//! Two-way bindings for forms (card web-bind, notes/web_framework.md step 6): `input{ bind: name }` shows name as its
//! value and sets name from what is typed (its handler of the page event input); a checkbox binds `checked`
use warp::wasm_emitter::eval;
use warp::web::markup::to_html;

fn html_of(code: &str) -> String {
	to_html(&eval(code))
}

#[test]
fn a_bound_input_shows_its_variable_and_handles_input() {
	assert_eq!(
		html_of("name = \"Ann\"\ndiv{ input{ bind: name } p{ \"Hi \" + name } }"),
		"<div><input value=\"Ann\" data-warp-input=\"1\"><p>Hi Ann</p></div>"
	);
}

#[test]
fn a_bound_checkbox_is_checked_by_its_variable() {
	assert_eq!(
		html_of("done = true\ninput{ type: \"checkbox\" bind: done }"),
		"<input type=\"checkbox\" checked data-warp-input=\"1\">"
	);
	assert_eq!(
		html_of("done = false\ninput{ type: \"checkbox\" bind: done }"),
		"<input type=\"checkbox\" data-warp-input=\"1\">"
	);
}

#[test]
fn boolean_attributes_are_present_or_absent() {
	assert_eq!(html_of("button{ disabled: true \"Go\" }"), "<button disabled>Go</button>");
	assert_eq!(html_of("button{ disabled: false \"Go\" }"), "<button>Go</button>");
}

// a form's submit is a page event whose event.fields holds its named fields (card web-submit)
#[cfg(feature = "native")]
#[test]
fn a_submitted_form_hands_its_fields_to_its_handler() {
	let program = "saved = \"\"\ndiv{ form{ on submit { saved = event.fields.name + \" is \" + event.fields.age } input{ name: \"name\" aria-label: \"name\" } input{ name: \"age\" type: \"number\" aria-label: \"age\" } button{ \"Save\" } } p{ saved } }";
	assert!(html_of(program).contains("<form data-warp-submit=\"1\">"), "{}", html_of(program));
	let mut page = warp::web::headless::Page::render(program).expect("renders");
	page.submit("Save", serde_json::json!({ "name": "Ann", "age": 7 })).expect("a form handles submit");
	assert!(page.text().ends_with("Ann is 7"), "{}", page.html());
}

// a handler on a line of its own inside a multi-line element is the element's handler, not data (card web-submit)
#[test]
fn a_handler_on_its_own_line_is_the_elements() {
	assert_eq!(
		html_of("y = 0\nform{\n\ton submit { y = 1 }\n\tbutton{ \"Add\" }\n}"),
		"<form data-warp-submit=\"1\"><button>Add</button></form>"
	);
	assert_eq!(
		html_of("y = 0\ndiv{\n\tbutton{ \"Add\" }\n\ton click { y = 1 }\n}"),
		"<div data-warp-click=\"1\"><button>Add</button></div>"
	);
}

// a built site carries markup-forms.js, after markup.js, only when a form has `on submit`: the hello-world budget
// (test_bundle_budget) pays nothing for it
#[test]
#[cfg(feature = "native")] // src/web/site.rs is native
fn a_site_with_a_submit_handler_carries_the_forms_script() {
	let scripts = |code: &str| {
		let directory = crate::common::scratch_directory("forms-site");
		let site = warp::web::site::build(code, "form", &directory).expect("the site is built");
		std::fs::remove_dir_all(directory).unwrap();
		site.files
	};
	let with = scripts("saved = \"\"\ndiv{ form{ on submit { saved = event.fields.name } input{name:\"name\" aria-label:\"name\"} button{\"Save\"} } p{ saved } }");
	assert_eq!(with[with.len() - 3..], ["markup.js", "markup-forms.js", "site.js"]);
	assert!(!scripts("div{ p{ \"hello\" } }").contains(&"markup-forms.js".to_string()));
}

// `select{ bind: choice }` shows the chosen option from its first render: the option whose value, else text, is the
// select's value is selected (card web-bind-select)
#[test]
fn a_bound_select_shows_the_chosen_option() {
	assert_eq!(html_of("fruit = \"pear\"\nselect{ bind: fruit aria-label: \"fruit\" option{ \"apple\" } option{ \"pear\" } }"),
		"<select value=\"pear\" data-warp-input=\"1\" aria-label=\"fruit\"><option>apple</option><option selected>pear</option></select>");
	assert_eq!(html_of("size = 2\nselect{ bind: size aria-label: \"size\" option{ value: 1 \"small\" } option{ value: 2 \"large\" } }"),
		"<select value=\"2\" data-warp-input=\"1\" aria-label=\"size\"><option value=\"1\">small</option><option value=\"2\" selected>large</option></select>");
}

// a bound field takes its checks from the variable's declared type: the browser refuses 2.5 for an int, a bool is a
// checkbox; what the element says itself wins (card web-validate)
#[test]
fn a_bound_field_is_validated_by_the_declared_type() {
	assert_eq!(
		html_of("age: int = 30\ninput{ bind: age aria-label: \"age\" }"),
		"<input type=\"number\" step=\"1\" value=\"30\" data-warp-input=\"1\" aria-label=\"age\">"
	);
	assert_eq!(
		html_of("price: float = 2.5\ninput{ bind: price aria-label: \"price\" }"),
		"<input type=\"number\" step=\"any\" value=\"2.5\" data-warp-input=\"1\" aria-label=\"price\">"
	);
	assert_eq!(
		html_of("done: bool = yes\ninput{ bind: done aria-label: \"done\" }"),
		"<input type=\"checkbox\" checked data-warp-input=\"1\" aria-label=\"done\">"
	);
	assert_eq!(
		html_of("age: int = 30\ninput{ bind: age type: \"range\" aria-label: \"age\" }"),
		"<input step=\"1\" value=\"30\" data-warp-input=\"1\" type=\"range\" aria-label=\"age\">"
	);
}

// `int age = 30` declares as `age: int = 30` does: a handler changes it, a bound field takes its checks (card
// bind-ignores)
#[test]
#[cfg(feature = "native")]
fn a_c_style_declared_variable_is_bound() {
	let code = "int age = 30\nint clicks = 0\ndiv{ input{ bind: age aria-label: \"age\" } button{ on click { clicks += 1 } \"More\" } p{ \"\" + age + \"/\" + clicks } }";
	assert!(html_of(code).contains("<input type=\"number\" step=\"1\" value=\"30\""), "{}", html_of(code));
	let mut page = warp::web::headless::Page::render(code).expect("the page");
	page.type_into("age", "31").expect("typed");
	page.click("More").expect("clicked");
	assert!(page.text().ends_with("31/1"), "{}", page.html());
}

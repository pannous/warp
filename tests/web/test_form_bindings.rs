//! Two-way bindings for forms (card web-bind, notes/web_framework.md step 6): `input{ bind: name }` shows name as its
//! value and sets name from what is typed (its handler of the page event input); a checkbox binds `checked`
use warp::markup::to_html;
use warp::wasm_emitter::eval;

fn html_of(code: &str) -> String {
	to_html(&eval(code))
}

#[test]
fn a_bound_input_shows_its_variable_and_handles_input() {
	assert_eq!(html_of("name = \"Ann\"\ndiv{ input{ bind: name } p{ \"Hi \" + name } }"), "<div><input value=\"Ann\" data-warp-input=\"1\"><p>Hi Ann</p></div>");
}

#[test]
fn a_bound_checkbox_is_checked_by_its_variable() {
	assert_eq!(html_of("done = true\ninput{ type: \"checkbox\" bind: done }"), "<input type=\"checkbox\" checked data-warp-input=\"1\">");
	assert_eq!(html_of("done = false\ninput{ type: \"checkbox\" bind: done }"), "<input type=\"checkbox\" data-warp-input=\"1\">");
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
	let mut page = warp::headless::Page::render(program).expect("renders");
	page.submit("Save", serde_json::json!({ "name": "Ann", "age": 7 })).expect("a form handles submit");
	assert!(page.text().ends_with("Ann is 7"), "{}", page.html());
}

// a handler on a line of its own inside a multi-line element is the element's handler, not data (card web-submit)
#[test]
fn a_handler_on_its_own_line_is_the_elements() {
	assert_eq!(html_of("y = 0\nform{\n\ton submit { y = 1 }\n\tbutton{ \"Add\" }\n}"), "<form data-warp-submit=\"1\"><button>Add</button></form>");
	assert_eq!(html_of("y = 0\ndiv{\n\tbutton{ \"Add\" }\n\ton click { y = 1 }\n}"), "<div data-warp-click=\"1\"><button>Add</button></div>");
}

// a built site carries markup-forms.js, after markup.js, only when a form has `on submit`: the hello-world budget
// (test_bundle_budget) pays nothing for it
#[test]
#[cfg(feature = "native")] // src/site.rs is native
fn a_site_with_a_submit_handler_carries_the_forms_script() {
	let scripts = |code: &str| {
		let directory = crate::common::scratch_directory("forms-site");
		let site = warp::site::build(code, "form", &directory).expect("the site is built");
		std::fs::remove_dir_all(directory).unwrap();
		site.files
	};
	let with = scripts("saved = \"\"\ndiv{ form{ on submit { saved = event.fields.name } input{name:\"name\" aria-label:\"name\"} button{\"Save\"} } p{ saved } }");
	assert_eq!(with[with.len() - 3..], ["markup.js", "markup-forms.js", "site.js"]);
	assert!(!scripts("div{ p{ \"hello\" } }").contains(&"markup-forms.js".to_string()));
}

//! Headless component tests (card web-testing, src/headless.rs): render a program whose value is markup, click a
//! button by its text, check what the page shows, natively without a browser
use warp::headless::Page;

const COUNTER: &str = "count = 0\ndiv{ button{ on click { count += 1 } \"Add\" } p{ \"clicked \" + count } }";

#[test]
fn a_click_runs_the_handler_and_shows_the_page_anew() {
	let mut page = Page::render(COUNTER).expect("renders");
	assert_eq!(page.text(), "Addclicked 0");
	page.click("Add").expect("clicks");
	page.click("Add").expect("clicks");
	assert_eq!(page.html(), "<div><button data-warp-click=\"1\">Add</button><p>clicked 2</p></div>");
}

#[test]
fn each_component_instance_counts_on_its_own() {
	let code = "def Counter(start) {\n\tcount = start\n\tdiv{ button{ on click { count += 1 } \"+\" } p{ \"n \" + count } }\n}\nsection{ Counter(1) Counter(5) }";
	let mut page = Page::render(code).expect("renders");
	page.click("+").expect("clicks the first");
	assert_eq!(page.text(), "+n 2+n 5");
}

#[test]
fn typing_into_a_bound_field_sets_its_variable() {
	let mut page = Page::render("name = \"\"\ndiv{ input{ placeholder: \"name\" bind: name } p{ \"hi \" + name } }").expect("renders");
	page.type_into("name", "Ada").expect("types");
	assert!(page.html().ends_with("<p>hi Ada</p></div>"), "{}", page.html());
}

#[test]
fn a_missing_button_is_a_loud_error() {
	let mut page = Page::render(COUNTER).expect("renders");
	let problem = page.click("Remove").expect_err("no such button");
	assert!(problem.contains("no element showing \"Remove\" handles click"), "{problem}");
}

//! Components are functions returning markup (card web-components, notes/web_framework.md step 4): props are its
//! parameters, positional or named, and a block after the call is its last argument, the children
use warp::wasm_emitter::eval;
use warp::web::markup::to_html;

const CARD: &str = "def Card(title, children) = div{ class:\"card\" h2{ title } children }\n";

fn html_of(code: &str) -> String {
	to_html(&eval(code))
}

#[test]
fn a_component_is_a_function_returning_markup() {
	assert_eq!(
		html_of("def Card(title) = div{ h2{ title } }\ndiv{ Card(\"A\") Card(title:\"B\") }"),
		"<div><div><h2>A</h2></div><div><h2>B</h2></div></div>"
	);
}

#[test]
fn the_block_after_the_call_is_the_children() {
	let card = "<div class=\"card\"><h2>Hi</h2><p>text</p><em>x</em></div>";
	assert_eq!(html_of(&format!("{CARD}Card(\"Hi\") {{ p:\"text\" em:\"x\" }}")), card);
	assert_eq!(html_of(&format!("{CARD}Card(title:\"Hi\") {{ p:\"text\" em:\"x\" }}")), card);
	assert_eq!(
		html_of(&format!("{CARD}section{{ Card(\"Hi\") {{ p:\"text\" em:\"x\" }} }}")),
		format!("<section>{card}</section>")
	);
}

// each instance keeps its own state (component_state.rs): the page passes the clicked instance as event.instance
// (tour example "components", test_in_browser.py --examples clicks it)
const COUNTER: &str =
	"def Counter(start) {\n\tcount = start\n\tdiv{ button{ on click { count += 1 } \"+\" } p{ \"n \" + count } }\n}\n";

#[test]
fn each_instance_of_a_component_has_its_own_state() {
	let counter = |instance: usize, count: usize| {
		format!("<div><button data-warp-instance=\"{instance}\" data-warp-click=\"1\">+</button><p>n {count}</p></div>")
	};
	assert_eq!(
		html_of(&format!("{COUNTER}div{{ Counter(1) Counter(5) }}")),
		format!("<div>{}{}</div>", counter(1, 1), counter(2, 5))
	);
	assert_eq!(html_of(&format!("{COUNTER}Counter(3)")), counter(1, 3));
}

#[test]
fn a_trailing_block_fills_a_function_parameter() {
	assert_eq!(eval("def apply(x, f) = f(x)\napply(3) { it * 2 }").serialize(), "6");
}

// `on mount {…}` runs when an instance first renders, `on cleanup {…}` when a later render leaves it out (card
// web-cleanup; tour example "cleanup" removes one)
#[test]
fn on_mount_runs_once_per_new_instance() {
	let code = "mounted = 0\ndef Tag(label) {\n\ton mount { mounted += 1 }\n\ton cleanup { mounted -= 1 }\n\tspan{ label }\n}\ndiv{ Tag(\"a\") Tag(\"b\") p{ \"mounted \" + mounted } }";
	assert_eq!(html_of(code), "<div><span>a</span><span>b</span><p>mounted 2</p></div>");
}

// `bind:` inside a component binds the instance's own state: typing into one Greeter changes only its greeting
// (card web-bind-component)
#[test]
#[cfg(feature = "native")]
fn a_bound_field_in_a_component_binds_its_instance() {
	let greeter = "def Greeter(start) {\n\tname = start\n\tdiv{ input{ bind: name placeholder: \"name\" } p{ \"Hi \" + name } }\n}\n";
	let mut page = warp::web::headless::Page::render(&format!("{greeter}div{{ Greeter(\"you\") Greeter(\"there\") }}"))
		.expect("the page");
	page.type_into("name", "Ann").expect("typed");
	assert!(page.text().contains("Hi Ann") && page.text().contains("Hi there"), "{}", page.html());
}

// A component's handler reads and writes the program's own variables: each Counter counts on its own by the shared step,
// and the page total counts every click (card web-outside-state)
#[test]
#[cfg(feature = "native")]
fn a_component_handler_changes_state_outside_it() {
	let counter = "step = 2\ntotal = 0\ndef Counter(start) {\n\tcount = start\n\tdiv{ button{ on click { count += step; total += 1 } \"+\" } p{ \"n \" + count } }\n}\n";
	let mut page = warp::web::headless::Page::render(&format!(
		"{counter}div{{ Counter(0) Counter(10) p{{ \"total \" + total }} }}"
	))
	.expect("the page");
	page.click("+").expect("clicked");
	assert!(
		page.text().contains("n 2") && page.text().contains("n 10") && page.text().contains("total 1"),
		"{}",
		page.html()
	);
}

// Outside its component a state variable is the list of every instance's value, `Counter.count#1` the first's: the
// page's own button reads and resets it (card web-outside-state)
#[test]
#[cfg(feature = "native")]
fn a_handler_outside_a_component_reads_its_state() {
	let counter = "def Counter(start) {\n\tcount = start\n\tdiv{ button{ on click { count += 1 } \"+\" } p{ \"n \" + count } }\n}\n";
	let page_markup = "div{ Counter(0) Counter(10) button{ on click { Counter.count#1 = 0 } \"Reset\" } p{ \"sum \" + sum(Counter.count) } }";
	let mut page = warp::web::headless::Page::render(&format!("{counter}{page_markup}")).expect("the page");
	page.click("+").expect("clicked");
	assert!(page.text().contains("sum 11"), "{}", page.html());
	page.click("Reset").expect("reset");
	assert!(page.text().contains("n 0") && page.text().contains("sum 10"), "{}", page.html());
}

// `key:` on a component call ties its state to the key, not to the position: after a swap the counted item keeps its
// count (card web-instance-keys)
#[test]
#[cfg(feature = "native")]
fn keyed_component_state_survives_reordering() {
	let counter = "def Counter(name) {\n\tcount = 0\n\tdiv{ button{ on click { count += 1 } \"+\" } p{ name + \" \" + count } }\n}\nitems = [\"a\", \"b\"]\n";
	let page_markup = "div{ button{ on click { items = [items#2, items#1] } \"Swap\" } [Counter(item, key: item) for item in items] }";
	let mut page = warp::web::headless::Page::render(&format!("{counter}{page_markup}")).expect("the page");
	page.click("+").expect("clicked");
	page.click("Swap").expect("swapped");
	let text = page.text();
	assert!(text.contains("a 1") && text.contains("b 0") && text.find("b 0") < text.find("a 1"), "{}", page.html());
}

// A keyed instance gone from the middle takes its state along: the ones after it keep theirs (card web-instance-keys)
#[test]
#[cfg(feature = "native")]
fn keyed_component_state_survives_a_removal() {
	let counter = "def Counter(name) {\n\tcount = 0\n\tdiv{ button{ on click { count += 1 } name } p{ name + \" \" + count } }\n}\nitems = [\"a\", \"b\", \"c\"]\n";
	let page_markup = "div{ button{ on click { items = [items#2, items#3] } \"Drop a\" } [Counter(item, key: item) for item in items] }";
	let mut page = warp::web::headless::Page::render(&format!("{counter}{page_markup}")).expect("the page");
	page.click("b").expect("clicked");
	page.click("Drop a").expect("dropped");
	let text = page.text();
	assert!(!text.contains("a 0") && text.contains("b 1") && text.contains("c 0"), "{}", page.html());
}

// `counters`, the lower-case plural, is Counter's instances in page order: `counters#1.count` the first one's count,
// `counters.count` every one's; `Counter.instances` is the long form (card web-outside-state, user decision)
#[test]
#[cfg(feature = "native")]
fn a_components_plural_names_its_instances() {
	let counter = "def Counter(start) {\n\tcount = start\n\tdiv{ button{ on click { count += 1 } \"+\" } p{ \"n \" + count } }\n}\n";
	let page_markup = "div{ Counter(0) Counter(10) button{ on click { counters#1.count = 0 } \"Reset\" } p{ \"sum \" + sum(counters.count) } p{ \"last \" + Counter.instances#2.count } }";
	let mut page = warp::web::headless::Page::render(&format!("{counter}{page_markup}")).expect("the page");
	page.click("+").expect("clicked");
	assert!(page.text().contains("sum 11") && page.text().contains("last 10"), "{}", page.html());
	page.click("Reset").expect("reset");
	assert!(page.text().contains("n 0") && page.text().contains("sum 10"), "{}", page.html());
}

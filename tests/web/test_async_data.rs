// card web-async: `users := fetch url` is a value that arrives later: ø with users.loading true until the reply is in,
// then the parsed JSON (else the text) or users.error; `on change users {…}` sees it, and a URL reading a variable is
// fetched anew when that changes (src/lowering/fetch_signals.rs, src/fetches.rs). The waits poll with sleep, whose
// check points run the arrived handler.
use crate::common::serve;

const USERS: &str = r#"[{"name": "Ann", "age": 31}, {"name": "Bo", "age": 7}]"#;
const UNTIL_ARRIVED: &str = "while users.loading { sleep 5 ms }";

fn program(url: &str, rest: &str) -> String {
	format!("users := fetch \"{url}\"\n{rest}")
}

#[test]
fn the_parsed_json_arrives_later() {
	let url = serve("200 OK", USERS);
	crate::is!(&program(&url, "users.loading"), true);
	crate::is!(&program(&url, &format!("{UNTIL_ARRIVED}\nusers#2.name")), "Bo");
	crate::is!(&program(&url, &format!("{UNTIL_ARRIVED}\nusers.error")), warp::node::Node::Empty);
}

#[test]
fn on_change_sees_the_arrived_value() {
	let url = serve("200 OK", USERS);
	crate::is!(&program(&url, &format!("seen = 0\non change users {{ seen = count(users) }}\n{UNTIL_ARRIVED}\nseen")), 2);
}

#[test]
fn a_failed_fetch_is_the_error() {
	let url = serve("404 Not Found", "no such users");
	crate::is!(&program(&url, &format!("{UNTIL_ARRIVED}\ncontains(users.error, \"HTTP status 404\")")), true);
}

#[test]
fn a_body_that_is_no_json_is_the_text() {
	let url = serve("200 OK", "hello warp");
	crate::is!(&program(&url, &format!("{UNTIL_ARRIVED}\nusers")), "hello warp\n");
}

#[test]
fn a_changed_url_variable_fetches_anew() {
	let url = serve("200 OK", USERS);
	let separator = if url.contains('?') { '&' } else { '?' }; // the browser's stub URL has a query already
	let code = format!("page = 1\nusers := fetch \"{url}{separator}page=\" + page\n{UNTIL_ARRIVED}\npage = 2\nagain = users.loading\n{UNTIL_ARRIVED}\nagain and count(users) == 2");
	crate::is!(&code, true);
}

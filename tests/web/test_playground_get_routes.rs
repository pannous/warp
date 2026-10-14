// cards square-int, recognized-browser: the playground has no server, so a top-level `get "/path" {…}` is also the page
// at that path there, as `route "/path" {…}` is: `get "/" {"ok"}` shows "ok", `get "/square/:n:int" { n*n }` shows 81
// at /square/9 (the address bar). Before, the page said "no page at /" or nothing.
use std::collections::HashSet;
use warp::host::with_page_path;
use warp::web::evaluate;

fn shown_at(path: &str, program: &str) -> String {
	with_page_path(path, || {
		evaluate(program, HashSet::new())["value"]
			.as_str()
			.unwrap_or_default()
			.to_string()
	})
}

#[test]
fn a_get_route_is_the_playground_page_at_its_path() {
	assert_eq!(shown_at("/", r#"get "/" {"ok"}"#), r#""ok""#);
	assert_eq!(shown_at("/square/9", r#"get "/square/:n:int" { n*n }"#), "81");
}

#[test]
fn get_and_page_routes_share_the_address_bar() {
	let program = r#"route "/" { a{href:"/square/3" "try it"} }
get "/square/:n:int" { n*n }"#;
	assert_eq!(shown_at("/square/3", program), "9");
	assert!(shown_at("/", program).contains("try it"));
}

// samples/route_headers.warp: a route's status and header lines have no HTTP answer in the playground, its page shows
#[test]
fn a_page_of_a_route_with_headers_shows_its_value() {
	let program = include_str!("../../samples/route_headers.warp");
	assert_eq!(shown_at("/teapot", program), r#""short and stout""#);
}

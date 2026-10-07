// card web-router (notes/web_framework.md "Routes"): `route "/users/:id" {…}` shows its block when the page's path
// matches, id bound to that part of the path (a number when it is digits); `route "*"` is the not-found page; a layout
// shows the matched route where it says `outlet`. Natively the path is "/" unless a render asks for another one.
use warp::host::with_page_path;
use warp::markup::to_html;
use warp::wasm_emitter::eval;

const SITE: &str = "users = [\"Ann\", \"Bo\"]
route \"/\" { h1{ \"Home\" } }
route \"/users/:id\" { p{ \"User \" + users#id } }
route \"*\" { p{ \"not found\" } }";

fn page_at(path: &str, program: &str) -> String {
	with_page_path(path, || to_html(&eval(program)))
}

#[test]
fn the_route_matching_the_path_is_shown() {
	assert_eq!(page_at("/", SITE), "<h1>Home</h1>");
	assert_eq!(page_at("/users/2", SITE), "<p>User Bo</p>");
	assert_eq!(page_at("/somewhere/else", SITE), "<p>not found</p>");
}

#[test]
fn without_a_matching_route_the_page_says_so() {
	assert_eq!(with_page_path("/nope", || eval("route \"/\" { p{ \"home\" } }")), warp::node::Node::Text("no page at /nope".into()));
}

#[test]
fn a_layout_shows_the_route_at_its_outlet() {
	let program = "route \"/\" { p{ \"home\" } }\nroute \"/about\" { p{ \"about\" } }\ndiv{ nav{ a{ href: \"/about\" \"About\" } } outlet }";
	assert_eq!(page_at("/about", program), "<div><nav><a href=\"/about\">About</a></nav><p>about</p></div>");
}

// a site loads a route's own module before showing it (card web-bundle): page·route_index is the index of the route the
// path picks, -1 when none
#[test]
fn the_route_index_names_the_route_the_path_picks() {
	let lowered = warp::pipeline::lower(SITE).expect("lowers").serialize();
	assert!(lowered.contains("(page·route_index):="), "{lowered}");
	assert!(lowered.contains("routed_index=(page·route_index)"), "{lowered}");
}

// card route-typed: a parameter declares its type in the pattern (":id:int", ":price:float", ":name:text"), so the route's
// block computes with it; a path whose part does not fit the type matches no such route (the next one, else not found)
#[test]
fn a_typed_route_parameter_computes_with_its_type() {
	let program = "route \"/users/:id:int\" { p{ \"next \" + (id + 1) } }\nroute \"/tags/:name:text\" { p{ name + \"!\" } }\nroute \"/prices/:price:float\" { p{ \"twice \" + price * 2 } }\nroute \"*\" { p{ \"not found\" } }";
	assert_eq!(page_at("/users/7", program), "<p>next 8</p>");
	assert_eq!(page_at("/tags/42", program), "<p>42!</p>");
	assert_eq!(page_at("/prices/1.5", program), "<p>twice 3</p>");
	assert_eq!(page_at("/users/bo", program), "<p>not found</p>");
}

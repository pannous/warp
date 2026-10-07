//! Routes of a page (card web-router, notes/web_framework.md "Routes"): `route "/users/:id" { UserPage(id) }` shows its
//! block when the page's path (the host word page_path(), "/" natively) matches the pattern, `id` bound to that part of
//! the path (a number when it is digits). Each route becomes the function page·route·N, the exported page·routes gives
//! the patterns in order, page·route_index() the index of the first route matching the path (-1 when none; the site
//! loads that route's module first, card web-bundle), and page·routed() that route's value, else "no page at <path>"; `route "*"`
//! matches any path (the not-found page). The program shows page·routed() where it says `outlet` (a layout around the
//! routes), else as its last line when that is a route. The matching is std/router.wasp's.

use crate::event_signals::{function_with_globals, main_level_variables};
use crate::node::{Bracket, Node, Separator};
use crate::system_signals::call;
use std::collections::HashMap;

const ROUTE_WORD: &str = "route";
const ROUTE_PREFIX: &str = "page·route·";
/// The patterns of the routes in order, exported for the site's loader (warp-cf: a module per route, card web-bundle)
pub const PAGE_ROUTES: &str = "page·routes";
const PAGE_ROUTED: &str = "page·routed";
pub const PAGE_ROUTE_INDEX: &str = "page·route_index";
const ROUTER_MODULE_USE: &str = "use router";
/// Where a layout shows the matched route
const OUTLET: &str = "outlet";
const PARAMETER_MARK: &str = ":";
/// `id` of the pattern bound in the route's function
const PARAMETER_TEMPLATE: &str = "let parameter = route_parameter(pattern, page_path(), parameter_name)";
const PATH_TEMPLATE: &str = "let routed_path = page_path()";
const MATCH_TEMPLATE: &str = "if route_matches(pattern, routed_path) { return index }";
const NO_ROUTE: &str = "-1";
const INDEX_TEMPLATE: &str = "let routed_index = index_of_route()";
const CHOICE_TEMPLATE: &str = "if routed_index == index { return chosen() }";
const NOT_FOUND_TEMPLATE: &str = "\"no page at \" + page_path()";

/// A route as written: its pattern and the items of its block
type Route = (String, Vec<Node>);

pub fn lower(program: Node) -> Node {
	let (statements, bracket, separator) = crate::variable_signals::main_statements(&program);
	if !statements.iter().any(|statement| route(statement).is_some()) {
		return program;
	}
	let ends_with_route = statements.last().is_some_and(|last| route(last).is_some());
	let main_variables = main_level_variables(&statements);
	let routes: Vec<Route> = statements.iter().filter_map(route).collect();
	let routed = call(PAGE_ROUTED, vec![]);
	let outlet = HashMap::from([(OUTLET.to_string(), routed.clone())]);
	let mut lowered: Vec<Node> = vec![crate::wasp_parser::parse(ROUTER_MODULE_USE)];
	lowered.extend(routes.iter().enumerate().map(|(index, (pattern, body))| function_with_globals(&format!("{ROUTE_PREFIX}{index}"), false, &route_body(pattern, body), &main_variables)));
	lowered.push(function_with_globals(PAGE_ROUTES, false, &[patterns(&routes)], &main_variables));
	lowered.push(function_with_globals(PAGE_ROUTE_INDEX, false, &matching(&routes), &main_variables));
	lowered.push(function_with_globals(PAGE_ROUTED, false, &choice(&routes), &main_variables));
	lowered.extend(statements.iter().filter(|statement| route(statement).is_none()).map(|statement| crate::law::substitute(statement, &outlet)));
	if ends_with_route {
		lowered.push(routed);
	}
	Node::List(lowered, bracket, separator)
}

/// `route "/users/:id" {…}`: the pattern ("/" parses as a character) and the block's items
fn route(statement: &Node) -> Option<Route> {
	let Node::List(items, _, _) = statement.drop_meta() else { return None };
	let [word, pattern, block] = items.as_slice() else { return None };
	let Node::List(body, Bracket::Curly, _) = block.drop_meta() else { return None };
	let pattern = match pattern.drop_meta() {
		Node::Text(text) => text.clone(),
		Node::Char(character) => character.to_string(),
		_ => return None,
	};
	(word.drop_meta().name() == ROUTE_WORD).then(|| (pattern, body.clone()))
}

/// The route's block after a `let` for each parameter of its pattern
fn route_body(pattern: &str, body: &[Node]) -> Vec<Node> {
	let parameters = pattern.split('/').filter_map(|segment| segment.strip_prefix(PARAMETER_MARK));
	parameters.map(|name| template(PARAMETER_TEMPLATE, [("parameter", Node::Symbol(name.to_string())), ("pattern", text(pattern)), ("parameter_name", text(name))]))
		.chain(body.iter().cloned()).collect()
}

/// `["/", "/users/:id"]`
fn patterns(routes: &[Route]) -> Node {
	Node::List(routes.iter().map(|(pattern, _)| text(pattern)).collect(), Bracket::Square, Separator::Space)
}

/// The index of the first route whose pattern matches the path, else -1
fn matching(routes: &[Route]) -> Vec<Node> {
	let matches = routes.iter().enumerate().map(|(index, (pattern, _))| template(MATCH_TEMPLATE, [("pattern", text(pattern)), ("index", Node::int(index as i64))]));
	std::iter::once(crate::wasp_parser::parse(PATH_TEMPLATE)).chain(matches).chain([crate::wasp_parser::parse(NO_ROUTE)]).collect()
}

/// The value of the route page·route_index picks, else the not-found text
fn choice(routes: &[Route]) -> Vec<Node> {
	let index = template(INDEX_TEMPLATE, [("index_of_route", Node::Symbol(PAGE_ROUTE_INDEX.to_string()))]);
	let choices = (0..routes.len()).map(|index| template(CHOICE_TEMPLATE, [("index", Node::int(index as i64)), ("chosen", Node::Symbol(format!("{ROUTE_PREFIX}{index}")))]));
	std::iter::once(index).chain(choices).chain([crate::wasp_parser::parse(NOT_FOUND_TEMPLATE)]).collect()
}

fn template<const N: usize>(code: &str, bindings: [(&str, Node); N]) -> Node {
	crate::law::substitute(&crate::wasp_parser::parse(code), &bindings.into_iter().map(|(name, value)| (name.to_string(), value)).collect())
}

fn text(value: &str) -> Node {
	Node::Text(value.to_string())
}

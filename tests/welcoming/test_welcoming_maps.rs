//! Maps as newcomers from Python and JavaScript use them (field test: Dijkstra): quoted and unquoted keys are the
//! same key, keys can be variables, `{}` grows, and keys/values/has/get/in and `for k in m` work on any map.
use crate::is;
use crate::common::fails_with;
use warp::*;

#[test]
fn test_quoted_key_with_a_list_value() {
	is!(r#"graph = {"A": [1, 2]}; graph["A"][1]"#, 2);
	is!(r#"graph = {"A": [["B", 4]], "F": []}; graph["A"][0][1]"#, 4);
}

#[test]
fn test_quoted_and_unquoted_keys_are_the_same_key() {
	is!(r#"a = {"A": 1}; b = {A: 1}; a.A + b["A"]"#, 2);
}

#[test]
fn test_key_in_a_variable() {
	is!(r#"m = {A: 7}; k = "A"; m[k] + 1"#, 8);
	is!(r#"graph = {A: [[1, 4], [3, 2]]}; k = "A"; graph[k][1][1]"#, 2);
	is!(r#"m = {AB: 1}; k = "AB"; m[k]"#, 1);
}

#[test]
fn test_empty_map_grows() {
	is!(r#"dist = {}; dist["A"] = 5; dist["A"]"#, 5);
	is!(r#"fun f() { d = {}; d["A"] = 3; return d }; r = f(); r["A"]"#, 3);
	is!(r#"d = {}; for k in ["A", "B"] { d[k] = 1 }; d["A"] + d.B"#, 2);
}

#[test]
fn test_absent_key() {
	fails_with(r#"m = {A: 1}; m["Z"]"#, "no field Z"); // like Python's KeyError
	is!(r#"m = {A: 1}; m.get("Z")"#, Node::Empty);
	is!(r#"m = {"A": 1}; m.get("Z", 7) + m.get("A", 7)"#, 8);
}

#[test]
fn test_keys_values_has() {
	is!(r#"graph = {"A": 1, "B": 2}; graph.keys().length"#, 2);
	is!(r#"m = {"A": 3, B: 4}; vs = m.values(); s = 0; for v in vs { s = s + v }; s"#, 7);
	is!(r#"m = {"A": 1}; m.has("A")"#, 1);
	is!(r#"m = {"A": 1}; m.has("B")"#, 0);
}

#[test]
fn test_in_and_contains() {
	is!(r#"m = {"A": 1}; "A" in m"#, 1);
	is!(r#"v = ["A"]; "A" in v"#, 1);
	is!(r#"v = [1, 2, 3]; v.contains(2)"#, 1);
	is!(r#"v = [1, 2, 3]; v.includes(5)"#, 0);
	is!(r#"visited = ["A"]; !visited.contains("B")"#, 1);
	is!(r#"v = [1]; v.has(1) + 1"#, 2);
}

#[test]
fn test_for_in_a_map_walks_its_keys() {
	is!(r#"m = {A: 1, B: 2}; s = ""; for k in m { s = s + k }; s"#, "AB");
	is!(r#"m = {A: 5}; s = ""; for k in m { s = s + k }; s"#, "A");
}

#[test]
fn test_for_key_value_in_a_map() {
	is!(r#"m = {A: 1, B: 2}; s = 0; for k, v in m { s = s + v }; s"#, 3);
	is!(r#"m = {A: 1, B: 2}; s = ""; n = 0; for (k, v) in m { s = s + k; n = n + v }; s == "AB" and n == 3"#, 1);
}

#[test]
fn test_for_destructures_pairs() {
	is!("x = 0; for (r, c) in [[0, 1], [2, 3]] { x += r*10 + c }; x", 24);
}

#[test]
fn test_map_parameter() {
	is!(r#"fun f(g) { d = {}; nodes = g.keys(); for n in nodes { d[n] = 9 }; d["B"] }; f({A: 1, B: 2})"#, 9);
	is!(r#"fun f(g) { c = ""; for e in g["A"] { c = c + e[0] }; c }; f({A: [["B", 4]]})"#, "B");
	is!(r#"fun f(g) { c = 0; for e in g["A"] { c = c + e[1] }; c }; f({A: [["B", 4]], B: []})"#, 4);
}

#[test]
fn test_dijkstra_core() {
	is!(r#"
graph = {"A": [["B", 4], ["C", 2]], "B": [["D", 5]], "C": [["B", 1], ["D", 8], ["E", 10]], "D": [["E", 2], ["F", 6]], "E": [["F", 3]], "F": []}
fun dijkstra(graph, start) {
	dist = {}
	nodes = graph.keys()
	for node in nodes { dist[node] = 999999 }
	dist[start] = 0
	visited = [""]
	while visited.length < nodes.length + 1 {
		current = ""
		best = 999999
		for node in nodes {
			if !visited.contains(node) && dist[node] < best {
				best = dist[node]
				current = node
			}
		}
		if current == "" { break }
		visited.push(current)
		for edge in graph[current] {
			neighbor = edge[0]
			candidate = dist[current] + edge[1]
			if candidate < dist[neighbor] { dist[neighbor] = candidate }
		}
	}
	return dist["F"]
}
dijkstra(graph, "A")"#, 13);
}

#[test]
fn test_list_returned_by_a_function_holds_maps() {
	is!(r#"fun f() { return [{F: 13}, 2] }; r = f(); d = r[0]; "x:" + d["F"]"#, "x:13");
	is!(r#"fun f() { return [{F: 13}, 2] }; r = f(); d = r[0]; x = d["F"]; x + 1"#, 14);
}

#[test]
fn test_one_character_argument_is_a_text() {
	is!(r#"fun p(prev, t) { n = t; n = prev[n]; n }; p({B: "A"}, "B")"#, 'A');
	is!(r#"fun p(t) { n = t; n = "AB"; n }; p("B")"#, "AB");
}

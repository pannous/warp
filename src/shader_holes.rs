//! `$name` in a shader block (card shader-holes, P235b amended 2026-10-09): an explicit hole, as in sql templates.
//! The parser writes it as WGSL's `values.name` (the uniform the values map becomes, src/gpu.rs uniform_layout) and
//! marks the shader's text with the names; this pass gives every paint or gpu_render of that shader the entries
//! `name: name` in its values map, so the warp variable is read at each call. A map written out keeps its own entry
//! of a name. Bare WGSL names still never capture warp variables.
//! The built-in holes (card shader-builtins) $width, $height, $size, $time, $frame, $mouse, $mouse_down and $key get no entry unless the program
//! has a variable of that name: the host fills them at each render (src/gpu.rs builtin_values, host-gpu.js).

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

/// The attribute of a shader text listing its holes' names
pub const HOLES_ATTRIBUTE: &str = "shader_holes";
/// The WGSL name of the values map (src/gpu.rs, host-gpu.js)
pub const VALUES: &str = "values";
/// The words rendering a shader with a values map last: `paint(shader, width, height, values)`
const RENDERING_WORDS: [&str; 2] = [crate::host::PAINT, warp_runtime::host_words::GPU_RENDER];
const VALUES_INDEX: usize = 4;
/// Holes every shader has, filled by the host: the image's size in pixels, seconds since the first render, the renders
/// before this one, the pointer over the image (in its pixels), whether a button is down, the key held down (its
/// character's code point, the arrows 0xF700 up, 0xF701 down, 0xF702 left, 0xF703 right as on a Mac; 0: none)
pub const BUILTIN_HOLES: [&str; 8] = ["width", "height", "size", "time", "frame", "mouse", "mouse_down", "key"];

/// The WGSL of a shader block with each `$name` as `values.name`, and the names in order (none: no hole)
pub fn written(wgsl: &str) -> (String, Vec<String>) {
	let mut names: Vec<String> = vec![];
	let mut rest = wgsl;
	let mut text = String::new();
	while let Some(at) = rest.find('$') {
		text.push_str(&rest[..at]);
		let after = &rest[at + 1..];
		let length = after.find(|ch: char| !(ch.is_alphanumeric() || ch == '_')).unwrap_or(after.len());
		let name = &after[..length];
		if name.is_empty() || name.starts_with(|ch: char| ch.is_ascii_digit()) {
			text.push('$');
			rest = after;
			continue;
		}
		text.push_str(&format!("{VALUES}.{name}"));
		if !names.iter().any(|known| known == name) {
			names.push(name.to_string());
		}
		rest = &after[length..];
	}
	text.push_str(rest);
	(text, names)
}

/// The shader text of a block, marked with its holes
pub fn shader_text(wgsl: &str) -> Node {
	let (text, names) = written(wgsl);
	match names.is_empty() {
		true => Node::Text(text),
		false => Node::Text(text).with_attribute(HOLES_ATTRIBUTE, Node::List(names.into_iter().map(Node::Symbol).collect(), Bracket::Square, Separator::Space)),
	}
}

pub fn lower(program: Node) -> Node {
	let mut shaders = HashMap::new();
	let mut variables = HashSet::new();
	program.visit(&mut |node| {
		if let Node::Key(variable, Op::Assign | Op::Define, value) = node {
			if let Node::Symbol(name) = variable.drop_meta() {
				variables.insert(name.clone());
				if let Some(holes) = holes_of(value) {
					shaders.insert(name.clone(), holes);
				}
			}
		}
	});
	if shaders.is_empty() && !mentions_holes(&program) {
		return program;
	}
	let passed = |holes: Vec<String>| holes.into_iter().filter(|name| !BUILTIN_HOLES.contains(&name.as_str()) || variables.contains(name)).collect::<Vec<_>>();
	let shaders = shaders.into_iter().map(|(shader, holes)| (shader, passed(holes))).collect();
	with_values(program, &shaders, &passed)
}

fn holes_of(node: &Node) -> Option<Vec<String>> {
	match node.attribute(HOLES_ATTRIBUTE)?.drop_meta() {
		Node::List(names, _, _) => Some(names.iter().map(Node::name).collect()),
		_ => None,
	}
}

fn mentions_holes(program: &Node) -> bool {
	let mut found = false;
	program.visit(&mut |node| found |= holes_of(node).is_some());
	found
}

fn with_values(node: Node, shaders: &HashMap<String, Vec<String>>, passed: &impl Fn(Vec<String>) -> Vec<String>) -> Node {
	let node = node.map_children(|child| with_values(child, shaders, passed));
	let Node::List(items, bracket, separator) = node.drop_meta() else { return node };
	let rendering = items.first().is_some_and(|word| RENDERING_WORDS.contains(&word.name().as_str()));
	let holes = items.get(1).filter(|_| rendering).and_then(|shader| holes_of(shader).map(passed).or_else(|| shaders.get(&shader.drop_meta().name()).cloned()));
	let Some(holes) = holes.filter(|holes| !holes.is_empty() && items.len() >= VALUES_INDEX - 1 && items.len() <= VALUES_INDEX + 1) else { return node };
	let mut items = items.clone();
	let entries = |given: &[Node]| holes.iter().filter(|name| !given.iter().any(|entry| entry_name(entry).as_deref() == Some(name.as_str()))).map(|name| entry(name)).collect::<Vec<_>>();
	match items.get(VALUES_INDEX).map(|values| values.drop_meta().clone()) {
		None => items.push(Node::List(entries(&[]), Bracket::Curly, Separator::Colon)),
		Some(Node::List(given, Bracket::Curly, map_separator)) => {
			let added = entries(&given);
			items[VALUES_INDEX] = Node::List(given.into_iter().chain(added).collect(), Bracket::Curly, map_separator).with_meta_of(&items[VALUES_INDEX]);
		}
		Some(_) => {
			let names = holes.iter().map(|name| format!("${name}")).collect::<Vec<_>>().join(", ");
			let message = format!("the shader's holes {names} need the values map written out here, `{{{}}}`, to add their entries", holes.iter().map(|name| format!("{name}: {name}")).collect::<Vec<_>>().join(", "));
			return crate::diagnostic::Diagnostic::at(&items[VALUES_INDEX], message).into_error();
		}
	}
	Node::List(items, bracket.clone(), separator.clone()).with_meta_of(&node)
}

fn entry_name(entry: &Node) -> Option<String> {
	match entry.drop_meta() {
		Node::Key(key, Op::Colon, _) => Some(key.drop_meta().name()),
		_ => None,
	}
}

fn entry(name: &str) -> Node {
	Node::key(name, Node::Symbol(name.to_string()))
}

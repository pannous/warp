//! `$name` in a shader block (card shader-holes, P235b amended 2026-10-09): an explicit hole, as in sql templates.
//! The parser writes it as WGSL's `values.name` (the uniform the values map becomes, src/graphics/gpu.rs uniform_layout) and
//! marks the shader's text with the names; this pass gives every paint or gpu_render of that shader the entries
//! `name: name` in its values map, so the warp variable is read at each call. A map written out keeps its own entry
//! of a name. Bare WGSL names still never capture warp variables.
//! The built-in holes (card shader-builtins) $width, $height, $size, $time, $frame, $mouse, $mouse_down, $mouse_wheel and $key get no entry unless the program
//! has a variable of that name: the host fills them at each render (src/graphics/gpu.rs builtin_values, host-gpu.js).

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_constructor::entry_name;
use std::collections::{HashMap, HashSet};

/// The attribute of a shader text listing its holes' names
pub const HOLES_ATTRIBUTE: &str = "shader_holes";
/// The WGSL name of the values map (src/graphics/gpu.rs, host-gpu.js)
pub const VALUES: &str = "values";
/// The words rendering a shader with a values map last: `paint(shader, width, height, values)`
const RENDERING_WORDS: [&str; 2] = [crate::host::PAINT, warp_runtime::host_words::GPU_RENDER];
const VALUES_INDEX: usize = 4;
const SIZE_INDEX: usize = 2;
/// The program's own size words, which paint(shader) takes before the view's size
const SIZE_WORDS: [&str; 2] = ["width", "height"];
/// The canvas's size words (lib/draw.warp), which a program sizing the canvas paints its shaders at: `canvas(w, h)`,
/// and `canvas.height = 80`, which property_accessors made such a call
const CANVAS_SIZE_WORDS: [&str; 2] = ["canvas_width", "canvas_height"];
const CANVAS_WORDS: [&str; 3] = ["canvas", "canvas_width", "canvas_height"];
/// Holes every shader has, filled by the host: the image's size in pixels, seconds since the first render, the renders
/// before this one, the pointer over the image (in its pixels), whether a button is down, the wheel steps turned over
/// it since the start (up adds; system value mouse_wheel), the key held down (its
/// character's code point, the arrows 0xF700 up, 0xF701 down, 0xF702 left, 0xF703 right as on a Mac; 0: none)
/// and `$previous(at)`, the shader's own previous frame at a pixel (src/graphics/gpu.rs PREVIOUS_FRAME)
pub const BUILTIN_HOLES: [&str; 10] =
	["width", "height", "size", "time", "frame", "mouse", "mouse_down", "mouse_wheel", "key", PREVIOUS_HOLE];
pub const PREVIOUS_HOLE: &str = "previous";
/// other words for $previous, which stay the program's own names: a value missing under one of them hints at $previous
pub const PREVIOUS_WORDS: [&str; 3] = ["last", "feedback", "before"];

/// Underscore spellings of a built-in hole's member, aliases that hint at the dotted form (user 2026-10-14, card
/// values-number): `$mouse_x` is `$mouse.x`; and The Book of Shaders' uniform names (glslCanvas, GLSL Sandbox; card
/// compute-toys): `$u_time` is `$time`, `$u_resolution` `$size`, `$u_mouse` `$mouse`; `$wheel` is
/// `$mouse_wheel` (card shader-wheel)
pub const HOLE_ALIASES: [(&str, &str, &str); 6] = [
	("mouse_x", "mouse.x", POINTER_REASON),
	("mouse_y", "mouse.y", POINTER_REASON),
	("u_time", "time", BOOK_OF_SHADERS_REASON),
	("u_resolution", "size", BOOK_OF_SHADERS_REASON),
	("u_mouse", "mouse", BOOK_OF_SHADERS_REASON),
	("wheel", "mouse_wheel", WHEEL_REASON),
];
const POINTER_REASON: &str = "the pointer is the built-in hole $mouse, a vec2f";
const WHEEL_REASON: &str = "the wheel is the built-in hole $mouse_wheel, as the pointer is $mouse";
const BOOK_OF_SHADERS_REASON: &str = "The Book of Shaders' uniform names are aliases of warp's built-in holes";

/// The dotted form of an aliased hole (`mouse_x` → `mouse.x`), any other name itself
fn dotted(name: &str) -> &str {
	HOLE_ALIASES.iter().find(|(alias, ..)| *alias == name).map_or(name, |(_, dotted, _)| dotted)
}

/// what a shader missing the value `name` may have meant ("" for nothing)
pub fn missing_value_hint(name: &str) -> &'static str {
	match PREVIOUS_WORDS.contains(&name) {
		true => "; the shader's own previous frame is $previous(at)",
		false => "",
	}
}

/// The WGSL of a shader block with each `$name` as `values.name`, and the names in order (none: no hole)
pub fn written(wgsl: &str) -> (String, Vec<String>) {
	let mut names: Vec<String> = vec![];
	let mut rest = wgsl;
	let mut text = String::new();
	while let Some(at) = rest.find('$') {
		text.push_str(&rest[..at]);
		let after = &rest[at + 1..];
		let length = after.find(|ch: char| !is_name_character(ch)).unwrap_or(after.len());
		let name = &after[..length];
		if name.is_empty() || name.starts_with(|ch: char| ch.is_ascii_digit()) {
			text.push('$');
			rest = after;
			continue;
		}
		let read = dotted(name);
		text.push_str(&format!("{VALUES}.{read}"));
		let name = read.split('.').next().unwrap_or(read);
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
	hint_aliases(wgsl);
	let (text, names) = written(wgsl);
	match names.is_empty() {
		true => Node::Text(text),
		false => Node::Text(text).with_attribute(
			HOLES_ATTRIBUTE,
			Node::List(names.into_iter().map(Node::Symbol).collect(), Bracket::Square, Separator::Space),
		),
	}
}

/// `$mouse_x` works as `$mouse.x`, with a hint to write that
fn hint_aliases(wgsl: &str) {
	for (alias, dotted, reason) in HOLE_ALIASES {
		let written = format!("${alias}");
		if mentions(wgsl, &written) {
			crate::normalize::hint(&written, &format!("${dotted}"), reason);
		}
	}
}

/// Whether the text has `name` where no letter, digit or underscore continues it
pub fn mentions(text: &str, name: &str) -> bool {
	text.match_indices(name).any(|(at, _)| !text[at + name.len()..].starts_with(is_name_character))
}

fn is_name_character(character: char) -> bool {
	character.is_alphanumeric() || character == '_'
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
	let sizes = sizes(&program, &variables);
	let program = with_sizes(program, &sizes);
	if shaders.is_empty() && !mentions_holes(&program) {
		return program;
	}
	let passed = |holes: Vec<String>| {
		holes
			.into_iter()
			.filter(|name| !BUILTIN_HOLES.contains(&name.as_str()) || variables.contains(name))
			.collect::<Vec<_>>()
	};
	let shaders = shaders.into_iter().map(|(shader, holes)| (shader, passed(holes))).collect();
	with_values(program, &shaders, &passed)
}

/// The width and height a paint without them takes: the program's `width` and `height`, else the canvas's size when the
/// program uses the canvas, else the view's size
fn sizes(program: &Node, variables: &HashSet<String>) -> [Node; 2] {
	let own = SIZE_WORDS.iter().all(|word| variables.contains(*word));
	let words = if own {
		SIZE_WORDS
	} else if uses_the_canvas(program, variables) {
		CANVAS_SIZE_WORDS
	} else {
		[warp_runtime::host_words::VIEW_WIDTH, warp_runtime::host_words::VIEW_HEIGHT]
	};
	words.map(|word| Node::Symbol(word.to_string()))
}

/// Does the program name the canvas of lib/draw.warp (a variable of its own named canvas is no such use)
fn uses_the_canvas(program: &Node, variables: &HashSet<String>) -> bool {
	let mut uses = false;
	program.visit(&mut |node| {
		if let Node::Symbol(word) = node {
			uses |= CANVAS_WORDS.contains(&word.as_str()) && !variables.contains(word);
		}
	});
	uses
}

/// `paint(shader)` and `paint(shader, values)` as `paint(shader, width, height, values)`
fn with_sizes(node: Node, sizes: &[Node; 2]) -> Node {
	let node = node.map_children(|child| with_sizes(child, sizes));
	let Node::List(items, bracket, separator) = node.drop_meta() else {
		return node;
	};
	let rendering = renders(items);
	let without_size = match items.len() {
		2 => true,
		3 => items[SIZE_INDEX].is_block(),
		_ => false,
	};
	if !rendering || !without_size {
		return node;
	}
	let mut items = items.clone();
	items.splice(SIZE_INDEX..SIZE_INDEX, sizes.iter().cloned());
	Node::List(items, bracket.clone(), separator.clone()).with_meta_of(&node)
}

/// A call of paint or gpu_render: the word, then its arguments
fn renders(items: &[Node]) -> bool {
	matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if RENDERING_WORDS.contains(&word.as_str()))
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

fn with_values(
	node: Node,
	shaders: &HashMap<String, Vec<String>>,
	passed: &impl Fn(Vec<String>) -> Vec<String>,
) -> Node {
	let node = node.map_children(|child| with_values(child, shaders, passed));
	let Node::List(items, bracket, separator) = node.drop_meta() else {
		return node;
	};
	let rendering = renders(items);
	let holes = items
		.get(1)
		.filter(|_| rendering)
		.and_then(|shader| holes_of(shader).map(passed).or_else(|| shaders.get(&shader.drop_meta().name()).cloned()));
	let Some(holes) =
		holes.filter(|holes| !holes.is_empty() && items.len() >= VALUES_INDEX - 1 && items.len() <= VALUES_INDEX + 1)
	else {
		return node;
	};
	let mut items = items.clone();
	let entries = |given: &[Node]| {
		holes
			.iter()
			.filter(|name| !given.iter().any(|entry| entry_name(entry).as_deref() == Some(name.as_str())))
			.map(|name| entry(name))
			.collect::<Vec<_>>()
	};
	match items.get(VALUES_INDEX).map(|values| values.drop_meta().clone()) {
		None => items.push(Node::List(entries(&[]), Bracket::Curly, Separator::Comma)),
		Some(Node::List(given, Bracket::Curly, map_separator)) => {
			let added = entries(&given);
			items[VALUES_INDEX] = Node::List(given.into_iter().chain(added).collect(), Bracket::Curly, map_separator)
				.with_meta_of(&items[VALUES_INDEX]);
		}
		Some(_) => {
			let names = holes.iter().map(|name| format!("${name}")).collect::<Vec<_>>().join(", ");
			let message = format!(
				"the shader's holes {names} need the values map written out here, `{{{}}}`, to add their entries",
				holes.iter().map(|name| format!("{name}: {name}")).collect::<Vec<_>>().join(", ")
			);
			return crate::errors::diagnostic::Diagnostic::at(&items[VALUES_INDEX], message).into_error();
		}
	}
	Node::List(items, bracket.clone(), separator.clone()).with_meta_of(&node)
}

/// `name: name`; a system value's in parentheses, so its word is read rather than kept as data (`$mouse_x` is the
/// pointer, system_values.rs is_bare_word; card values-number)
fn entry(name: &str) -> Node {
	let word = Node::Symbol(name.to_string());
	let is_system_value = warp_runtime::host_words::SYSTEM_VALUES.iter().any(|(system, _)| *system == name);
	Node::key(name, if is_system_value { Node::List(vec![word], Bracket::Round, Separator::None) } else { word })
}

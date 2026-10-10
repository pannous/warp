//! A component's world (card wasm-interop-rest, samples/wasm_interop.warp):
//! `component my_component { import host: { print: (string) -> () } export api: calculator }` declares what the
//! component imports and exports. `warp build --wit` writes it as a WIT world, each warp interface (`interface calculator
//! { add: (i32, i32) -> i32 }`) as a WIT interface. The declaration does nothing when the program runs.

use crate::node::{Bracket, Node};
use crate::operators::Op;

const COMPONENT_WORD: &str = "component";
const IMPORT_WORD: &str = "import";
/// The parser reads `export name: …` as `global name: …`
const EXPORT_WORDS: [&str; 2] = ["export", "global"];
const INTERFACE_WORD: &str = "interface";
const PACKAGE_NAMESPACE: &str = "warp";
/// warp's type words in WIT
const WIT_TYPES: [(&str, &str); 16] = [
	("i8", "s8"), ("i16", "s16"), ("i32", "s32"), ("i64", "s64"), ("int", "s64"), ("u8", "u8"), ("u16", "u16"), ("u32", "u32"),
	("u64", "u64"), ("f32", "f32"), ("f64", "f64"), ("float", "f64"), ("bool", "bool"), ("char", "char"), ("string", "string"), ("text", "string"),
];
const INDENT: &str = "  ";

/// A function of an interface: its name, parameter types and result type (None for `()`), in WIT
#[derive(Clone)]
pub struct Signature {
	pub name: String,
	pub parameters: Vec<String>,
	pub result: Option<String>,
}

/// What a component imports or exports: the direction, its name and its functions
pub struct WorldItem {
	pub direction: &'static str,
	pub name: String,
	pub functions: Vec<Signature>,
}

/// A program's component: its name, the program's interfaces and what it imports and exports, in WIT names
pub struct World {
	pub name: String,
	pub interfaces: Vec<(String, Vec<Signature>)>,
	pub items: Vec<WorldItem>,
}

pub const EXPORT_DIRECTION: &str = "export";
pub const WIT_STRING: &str = "string";

/// The functions a component exports and imports, each with the name of its export or import (`api`, `host`)
#[derive(Clone, Default)]
pub struct WorldFunctions {
	pub exports: Vec<(String, Signature)>,
	pub imports: Vec<(String, Signature)>,
}

impl World {
	pub fn functions(&self) -> WorldFunctions {
		let of = |direction: &str| self.items.iter().filter(|item| item.direction == direction)
			.flat_map(|item| item.functions.iter().map(|function| (item.name.clone(), function.clone()))).collect();
		WorldFunctions { exports: of(EXPORT_DIRECTION), imports: of(IMPORT_WORD) }
	}
}

/// The program without its component declarations; compiled as a component, a call of a function its world imports
/// (`host.time()`) is the call of that import (`host.time`, a core import of the module `host`)
pub fn lower(node: Node) -> Node {
	let imports = crate::pipeline::component_imports();
	let program = without_components(node);
	match imports.is_empty() {
		true => program,
		false => import_calls(program, &imports),
	}
}

fn without_components(node: Node) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			Node::List(items.into_iter().filter(|item| component(item).is_none()).map(without_components).collect(), bracket, separator)
		}
		Node::Meta { node, data } => Node::Meta { node: Box::new(without_components(*node)), data },
		other => other,
	}
}

/// The name of the import `function` of the world's import `interface`, as the program calls it and the emitter
/// keys it: `host.time`
pub fn import_name(interface: &str, function: &str) -> String {
	format!("{interface}.{function}")
}

/// `interface.function(arguments)` of an imported function: the call `(interface.function arguments)`
fn import_calls(node: Node, imports: &[(String, Signature)]) -> Node {
	if let Node::Key(receiver, Op::Dot, member) = node.drop_meta() {
		let (function, arguments) = match member.drop_meta() {
			Node::List(items, Bracket::Round, _) => match items.split_first() {
				Some((Node::Symbol(function), arguments)) => (function.clone(), arguments.to_vec()),
				_ => (String::new(), vec![]),
			},
			_ => (String::new(), vec![]),
		};
		let interface = kebab(&receiver.drop_meta().name());
		if imports.iter().any(|(imported, signature)| *imported == interface && signature.name == kebab(&function)) {
			let arguments = arguments.into_iter().map(|argument| import_calls(argument, imports));
			let callee = Node::Symbol(import_name(&interface, &kebab(&function)));
			return Node::List(std::iter::once(callee).chain(arguments).collect(), Bracket::Round, crate::node::Separator::None);
		}
	}
	node.map_children(|child| import_calls(child, imports))
}

/// The world of the program's one component declaration
pub fn world(program: &Node) -> Result<World, String> {
	let mut components = vec![];
	let mut interfaces = vec![];
	program.visit(&mut |node| {
		components.extend(component(node));
		interfaces.extend(declaration(node, INTERFACE_WORD));
	});
	let [(name, body)] = components.as_slice() else {
		return Err(format!("{} component declarations: a world is declared by one", components.len()));
	};
	let interfaces: Vec<(String, Vec<Signature>)> = interfaces.iter().map(|(name, body)| Ok((kebab(name), signatures(body)?))).collect::<Result<_, String>>()?;
	let items = body.as_items().iter().map(|item| {
		let (direction, name, members) = world_item(item).ok_or_else(|| format!("a component declares `import name: {{…}}` or `export name: interface`, not {}", item.serialize()))?;
		let functions = match members.drop_meta() {
			Node::Symbol(interface) => signatures(&interface_body(program, interface).ok_or_else(|| format!("{direction} {name}: no interface {interface}"))?)?,
			block => signatures(block)?,
		};
		Ok(WorldItem { direction, name: kebab(&name), functions })
	}).collect::<Result<_, String>>()?;
	Ok(World { name: kebab(name), interfaces, items })
}

/// The WIT of the program's component: its package with the program's interfaces, and its world
pub fn world_wit(program: &Node) -> Result<String, String> {
	let world = world(program)?;
	let mut wit = format!("package {PACKAGE_NAMESPACE}:{};\n", world.name);
	for (name, functions) in &world.interfaces {
		wit += &format!("\n{}\n", block_text(&format!("interface {name}"), functions, ""));
	}
	wit += &format!("\nworld {} {{\n", world.name);
	for item in &world.items {
		wit += &format!("{INDENT}{}\n", block_text(&format!("{} {}: interface", item.direction, item.name), &item.functions, INDENT));
	}
	Ok(wit + "}\n")
}

/// The body of the interface `name` the program declares
fn interface_body(program: &Node, name: &str) -> Option<Node> {
	let mut found = None;
	program.visit(&mut |node| if let Some((declared, body)) = declaration(node, INTERFACE_WORD) {
		if declared == name {
			found = Some(body);
		}
	});
	found
}

/// `component name {…}`: its name and body
fn component(node: &Node) -> Option<(String, Node)> {
	declaration(node, COMPONENT_WORD)
}

/// `<word> name {…}`: its name and body
fn declaration(node: &Node, word: &str) -> Option<(String, Node)> {
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	let [keyword, name, body] = items.as_slice() else { return None };
	let declares = matches!(keyword.drop_meta(), Node::Symbol(keyword) if keyword == word) && matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _));
	let Node::Symbol(name) = name.drop_meta() else { return None };
	declares.then(|| (name.clone(), body.drop_meta().clone()))
}

/// `import host: {…}` or `export api: calculator`: the direction, the name and the members (a block or an interface name)
fn world_item(item: &Node) -> Option<(&'static str, String, Node)> {
	let named = |node: &Node| match node.drop_meta() {
		Node::Key(name, Op::Colon, members) => Some((name.drop_meta().name(), members.as_ref().clone())),
		_ => None,
	};
	match item.drop_meta() {
		Node::List(items, _, _) if items.len() == 2 && items[0].drop_meta().name() == IMPORT_WORD => named(&items[1]).map(|(name, members)| (IMPORT_WORD, name, members)),
		Node::Key(word, Op::Colon, export) if EXPORT_WORDS.contains(&word.drop_meta().name().as_str()) => named(export).map(|(name, members)| (EXPORT_DIRECTION, name, members)),
		_ => None,
	}
}

/// The members `name: (types) -> type` of a block
fn signatures(block: &Node) -> Result<Vec<Signature>, String> {
	block.as_items().iter().map(|member| signature(member).ok_or_else(|| format!("an interface member is `name: (types) -> type`, not {}", member.serialize()))).collect()
}

fn signature(member: &Node) -> Option<Signature> {
	let Node::Key(head, Op::Arrow, result) = member.drop_meta() else { return None };
	let Node::Key(name, Op::Colon, parameters) = head.drop_meta() else { return None };
	let parameters = parameters.as_items().iter().map(wit_type).collect::<Option<Vec<_>>>()?;
	let result = match result.drop_meta() {
		Node::Empty => None,
		other => Some(wit_type(other)?),
	};
	Some(Signature { name: kebab(&name.drop_meta().name()), parameters, result })
}

fn wit_type(node: &Node) -> Option<String> {
	let word = node.drop_meta().name();
	WIT_TYPES.iter().find(|(warp, _)| *warp == word).map(|(_, wit)| wit.to_string())
}


/// `header { name: func(p1: s32) -> s32; … }`, the members indented one level below `indent`
fn block_text(header: &str, functions: &[Signature], indent: &str) -> String {
	let lines: String = functions.iter().map(|function| format!("{indent}{INDENT}{}\n", function_text(function))).collect();
	format!("{header} {{\n{lines}{indent}}}")
}

fn function_text(function: &Signature) -> String {
	let parameters: Vec<String> = function.parameters.iter().enumerate().map(|(index, kind)| format!("p{}: {kind}", index + 1)).collect();
	let result = function.result.as_ref().map(|result| format!(" -> {result}")).unwrap_or_default();
	format!("{}: func({}){result};", function.name, parameters.join(", "))
}

/// WIT names are kebab-case: `my_component` is my-component
fn kebab(name: &str) -> String {
	name.replace('_', "-")
}

//! `ys = xs.map(x => …) @gpu` (card gpu-vectors, P214: floats go to the GPU only where the program allows f32): a pure
//! numeric lambda over a linear float array runs as a WGSL compute kernel in f32. shared_arrays.rs lowers the map to
//! the host word gpu_map_linear, which reads xs's block and writes ys's (src/host.rs natively, host-gpu.js in the
//! browser); without an adapter it says so once and gives 0, and the CPU maps the items instead. Where `@gpu` cannot
//! apply, a warning says why and the map runs on the CPU as written (an error under strict). notes/gpu.md

use crate::node::Node;
use crate::operators::Op;

const GPU_ATTRIBUTE: &str = "gpu";
/// Threads per workgroup of the kernel; gpu_map_linear dispatches count / WORKGROUP_SIZE workgroups, rounded up
pub(crate) const WORKGROUP_SIZE: i64 = 256;
/// Fewer items map on the CPU (card gpu-threshold, notes/gpu.md): the GPU's ~2 ms of setup and readback loses below
/// ~3·10^4 items even against a heavy lambda (sin, cos: ~70–190 ns an item on the CPU, ~12 on the GPU)
pub(crate) const GPU_MAP_MIN_COUNT: i64 = 32768;
/// The attributes marking an @gpu map whose result stays on the GPU for a later map of it, and that later map
const KEEP_RESULT_ATTRIBUTE: &str = "gpu_keep_result";
const SOURCE_KEPT_ATTRIBUTE: &str = "gpu_source_kept";
/// gpu_map_linear's and gpu_reduce_linear's last argument: keep the result's buffer on the GPU (under the target block's
/// address), take the source's items from such a buffer (src/gpu.rs KEPT; uploaded from memory when none is kept)
pub(crate) const KEEP_RESULT: i64 = 1;
pub(crate) const SOURCE_KEPT: i64 = 2;
/// The lambda's parameter as the kernel names it (the user's name could be a WGSL keyword)
const KERNEL_ITEM: &str = "item";
/// The math words WGSL has under the same name and meaning (warp's log is the natural logarithm, as WGSL's)
const KERNEL_FUNCTIONS: [&str; 15] = ["sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "exp", "log", "sqrt", "abs", "floor", "ceil"];
const KERNEL_FUNCTIONS_OF_TWO: [&str; 3] = ["min", "max", "atan2"];
/// The math words that cost the CPU tens of nanoseconds an item, so a lambda with one is worth the GPU
const HEAVY_FUNCTIONS: [&str; 12] = ["sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "exp", "log", "atan2"];
/// The kernel's count of items: `data` holds them, then the program's numbers the lambda reads
const KERNEL_COUNT: &str = "count";
/// `x^k` for a whole k up to this is a product (pow of a negative base is NaN in WGSL, a number on the CPU)
const LARGEST_MULTIPLIED_POWER: i64 = 8;

/// `xs.map(f) @gpu` or `@gpu xs.map(f)`: xs and f; a chain `xs.map(f).map(g)` is xs and g after f, one kernel
pub(crate) fn gpu_map(value: &Node) -> Option<(Node, Node)> {
	let (list, function) = crate::parallel::is_annotated(value, GPU_ATTRIBUTE).then(|| crate::parallel::map_call(value)).flatten()?;
	Some(fused(list, function))
}

/// The map of `list` by `function` with the maps `list` itself is made of folded in: `xs.map(f).map(g)` is xs and g
/// after f, one pass over xs without a list of f's results
pub(crate) fn fused(mut list: Node, mut function: Node) -> (Node, Node) {
	loop {
		let Some((inner_list, inner_function)) = crate::parallel::map_call(&list) else { return (list, function) };
		let Some(both) = composed(&inner_function, &function) else { return (list, function) };
		(list, function) = (inner_list, both);
	}
}

/// `x => g_body` after `x => f_body`: `x => g_body` with g's parameter replaced by f's body (none when g reads a name
/// f's parameter would capture)
fn composed(first: &Node, then: &Node) -> Option<Node> {
	let (Node::Key(parameter, Op::FatArrow, body), Node::Key(then_parameter, Op::FatArrow, then_body)) = (first.drop_meta(), then.drop_meta()) else { return None };
	let (Node::Symbol(name), Node::Symbol(then_name)) = (parameter.drop_meta(), then_parameter.drop_meta()) else { return None };
	if name != then_name && crate::warp_parser::mentions(then_body, name) {
		return None;
	}
	let grouped = Node::List(vec![body.as_ref().clone()], crate::node::Bracket::Round, crate::node::Separator::None);
	let body = crate::library_words::substitute(then_body.as_ref().clone(), then_name, &grouped);
	Some(Node::Key(parameter.clone(), Op::FatArrow, Box::new(body)))
}

/// `ys = xs.map(f) @gpu; zs = ys.map(g) @gpu`, ys read nowhere else: `zs = xs.map(f).map(g) @gpu`, so ys never comes
/// back from the GPU (one round trip, notes/gpu.md). Only while nothing the map of ys reads (xs, the lambda's numbers)
/// is written after it in the same statements
pub fn kept_on_gpu(node: Node) -> Node {
	let program = node.clone();
	kept_in(node, &program)
}

fn kept_in(node: Node, program: &Node) -> Node {
	let node = node.map_children(|child| kept_in(child, program));
	let Node::List(mut statements, bracket, separator @ (crate::node::Separator::Semicolon | crate::node::Separator::Newline)) = node else { return node };
	let mut index = 0;
	while index < statements.len() {
		match folded_into_consumers(&statements, index, program) {
			Some(rewritten) => statements = rewritten,
			None => index += 1,
		}
	}
	Node::List(marked_kept(statements), bracket, separator)
}

/// `ys = xs.map(f) @gpu; print ys#1; zs = ys.map(g) @gpu`: ys is read on the CPU, so it comes back, but its buffer stays on
/// the GPU too, and the map of ys starts from it instead of uploading ys again. Only while the statements between read
/// ys's items and nothing else (no write, no call handing ys on)
fn marked_kept(mut statements: Vec<Node>) -> Vec<Node> {
	for index in 0..statements.len() {
		let Some((name, value)) = gpu_assignment(&statements[index]) else { continue };
		// a map of only arithmetic runs on the CPU and leaves no buffer on the GPU
		if gpu_map(&value).and_then(|(_, function)| gpu_kernel(&function)).is_none() {
			continue;
		}
		let mut kept = false;
		for statement in &mut statements[index + 1..] {
			if maps_on_gpu(statement, &name) {
				*statement = with_attribute_on_value(statement.clone(), SOURCE_KEPT_ATTRIBUTE);
				kept = true;
			} else if !reads_only_items(statement, &name) {
				break;
			}
			if writes(statement, &name) {
				break;
			}
		}
		if kept {
			statements[index] = with_attribute_on_value(statements[index].clone(), KEEP_RESULT_ATTRIBUTE);
		}
	}
	statements
}

/// `zs = ys.map(g) @gpu` or `s = sum(ys.map(g) @gpu)` of ys
fn maps_on_gpu(statement: &Node, name: &str) -> bool {
	let Node::Key(target, Op::Assign, value) = statement.drop_meta() else { return false };
	let map = match gpu_reduction(value) {
		Some((_, map)) => map,
		None => value.as_ref().clone(),
	};
	matches!(target.drop_meta(), Node::Symbol(_)) && gpu_map(&map).is_some_and(|(list, _)| matches!(list.drop_meta(), Node::Symbol(source) if source == name))
}

/// Whether `node` uses `name` only for its items and count (`ys#i`, `#ys`, in text holes too)
fn reads_only_items(node: &Node, name: &str) -> bool {
	let holes = crate::interpolation::holes(node);
	if !holes.is_empty() {
		return holes.iter().all(|hole| reads_only_items(hole, name));
	}
	let is_name = |node: &Node| matches!(node.drop_meta(), Node::Symbol(symbol) if symbol == name);
	match node.drop_meta() {
		Node::Symbol(symbol) => symbol != name,
		Node::Key(list, Op::Hash, index) if is_name(list) => reads_only_items(index, name),
		Node::Key(empty, Op::Hash, list) if matches!(empty.drop_meta(), Node::Empty) && is_name(list) => true,
		Node::Key(left, _, right) => reads_only_items(left, name) && reads_only_items(right, name),
		Node::List(items, _, _) => items.iter().all(|item| reads_only_items(item, name)),
		_ => true,
	}
}

/// The assignment with `attribute` on its value
fn with_attribute_on_value(statement: Node, attribute: &str) -> Node {
	match statement {
		Node::Meta { node, data } => Node::Meta { node: Box::new(with_attribute_on_value(*node, attribute)), data },
		Node::Key(target, op, value) => Node::Key(target, op, Box::new(value.with_attribute(attribute, Node::True))),
		other => other,
	}
}

/// The flags of an @gpu map's value for the host word: KEEP_RESULT, SOURCE_KEPT
pub(crate) fn keeping(value: &Node) -> i64 {
	[(KEEP_RESULT_ATTRIBUTE, KEEP_RESULT), (SOURCE_KEPT_ATTRIBUTE, SOURCE_KEPT)].iter().filter(|(attribute, _)| value.attribute(attribute).is_some()).map(|(_, flag)| flag).sum()
}

/// The statements with statement `index`, an @gpu map whose result only later @gpu maps read, folded into those
fn folded_into_consumers(statements: &[Node], index: usize, program: &Node) -> Option<Vec<Node>> {
	let (name, value) = gpu_assignment(&statements[index])?;
	let later = &statements[index + 1..];
	let mut read = vec![];
	value.drop_meta().visit(&mut |part| if let Node::Symbol(symbol) = part { read.push(symbol.clone()) });
	if later.iter().any(|statement| read.iter().any(|symbol| writes(statement, symbol))) {
		return None;
	}
	let uses = uses_of(program, &name);
	let bare = value.drop_meta().clone();
	let mut consumers = 0;
	let rewritten: Vec<Node> = later.iter().map(|statement| match gpu_assignment(statement) {
		Some((_, consumer)) if crate::parallel::map_call(&consumer).is_some_and(|(list, _)| matches!(list.drop_meta(), Node::Symbol(source) if *source == name)) => {
			consumers += 1;
			crate::library_words::substitute(statement.clone(), &name, &bare)
		}
		_ => statement.clone(),
	}).collect();
	let all_fold = rewritten.iter().zip(later).filter(|(new, old)| new != old).all(|(new, _)| {
		gpu_assignment(new).and_then(|(_, value)| gpu_map(&value)).is_some_and(|(list, _)| crate::parallel::map_call(&list).is_none())
	});
	(consumers > 0 && uses == consumers + 1 && all_fold).then(|| [&statements[..index], &rewritten[..]].concat())
}

/// How often `name` occurs in `node`, in the holes of its texts too (`"\(ys#1)"` reads ys)
fn uses_of(node: &Node, name: &str) -> usize {
	let holes = crate::interpolation::holes(node);
	if !holes.is_empty() {
		return holes.iter().map(|hole| uses_of(hole, name)).sum();
	}
	match node.drop_meta() {
		Node::Symbol(symbol) => usize::from(symbol == name),
		Node::Key(left, _, right) => uses_of(left, name) + uses_of(right, name),
		Node::List(items, _, _) => items.iter().map(|item| uses_of(item, name)).sum(),
		_ => 0,
	}
}

/// `ys = xs.map(f) @gpu`: ys and the map
fn gpu_assignment(statement: &Node) -> Option<(String, Node)> {
	let Node::Key(target, Op::Assign, value) = statement.drop_meta() else { return None };
	let Node::Symbol(name) = target.drop_meta() else { return None };
	gpu_map(value).map(|_| (name.clone(), value.as_ref().clone()))
}

/// Whether `node` assigns `name` or one of its items
fn writes(node: &Node, name: &str) -> bool {
	let mut found = false;
	node.visit(&mut |part| if let Node::Key(target, op, _) = part {
		let assigned = match target.drop_meta() {
			Node::Key(list, Op::Hash, _) => list.drop_meta(),
			other => other,
		};
		found |= (*op == Op::Assign || op.is_compound_assign()) && matches!(assigned, Node::Symbol(symbol) if symbol == name);
	});
	found
}

/// A lambda as a WGSL compute kernel: the item it computes in WGSL, the program's numbers it reads (`outer`, appended
/// to `data` after the items, in this order) and whether it is worth the GPU (math words, powers: `heavy`)
pub(crate) struct Kernel {
	computed: String,
	pub outer: Vec<String>,
	heavy: bool,
}

/// The words reducing an @gpu map to one number: `s = sum(xs.map(x => …) @gpu)`
#[derive(Clone, Copy)]
pub(crate) enum Reduction {
	Sum,
	Min,
	Max,
}

const REDUCTIONS: [Reduction; 3] = [Reduction::Sum, Reduction::Min, Reduction::Max];
/// The largest f32, where min and max start (WGSL has no infinity literal), by its bits: the decimal 3.4028235e38
/// rounds above it, which Chrome's WGSL compiler (Tint) refuses
const LARGEST_F32: &str = "bitcast<f32>(0x7f7fffffu)";

impl Reduction {
	pub(crate) fn word(self) -> &'static str {
		match self {
			Reduction::Sum => "sum",
			Reduction::Min => "min",
			Reduction::Max => "max",
		}
	}

	/// The value no item changes, in WGSL
	fn identity(self) -> String {
		match self {
			Reduction::Sum => "0.0".into(),
			Reduction::Min => LARGEST_F32.into(),
			Reduction::Max => format!("-{LARGEST_F32}"),
		}
	}

	/// `left` and `right` combined, in WGSL and in warp
	pub(crate) fn combined(self, left: &str, right: &str) -> String {
		match self {
			Reduction::Sum => format!("({left} + {right})"),
			Reduction::Min => format!("min({left}, {right})"),
			Reduction::Max => format!("max({left}, {right})"),
		}
	}
}

/// `sum(xs.map(f) @gpu)`, min, max: the reduction and the @gpu map
pub(crate) fn gpu_reduction(value: &Node) -> Option<(Reduction, Node)> {
	let Node::List(items, _, _) = value.drop_meta() else { return None };
	let [word, argument] = items.as_slice() else { return None };
	let reduction = REDUCTIONS.into_iter().find(|reduction| word.drop_meta().name() == reduction.word())?;
	let argument = match argument.drop_meta() {
		Node::List(inner, crate::node::Bracket::Round, _) if inner.len() == 1 => &inner[0],
		_ => argument,
	};
	gpu_map(argument).map(|_| (reduction, argument.clone()))
}

const KERNEL_BINDING: &str = "@group(0) @binding(0) var<storage, read_write> data: array<f32>;";

impl Kernel {
	/// The shader mapping every item of `data` in place
	pub(crate) fn map_shader(&self) -> String {
		format!("{KERNEL_BINDING}
@compute @workgroup_size({WORKGROUP_SIZE}) fn main(@builtin(global_invocation_id) id: vec3<u32>) {{
	let {KERNEL_COUNT} = arrayLength(&data) - {}u;
	if (id.x < {KERNEL_COUNT}) {{ let {KERNEL_ITEM} = data[id.x]; data[id.x] = {}; }}
}}", self.outer.len(), self.computed)
	}

	/// The shader reducing the mapped items of each workgroup in its shared memory, halving the active threads each
	/// step, into one cell per workgroup after the items and the outer numbers
	pub(crate) fn reduce_shader(&self, reduction: Reduction) -> String {
		let outer = self.outer.len();
		format!("{KERNEL_BINDING}
var<workgroup> partial: array<f32, {WORKGROUP_SIZE}>;
@compute @workgroup_size({WORKGROUP_SIZE}) fn main(@builtin(global_invocation_id) id: vec3<u32>, @builtin(local_invocation_id) local: vec3<u32>, @builtin(workgroup_id) group: vec3<u32>, @builtin(num_workgroups) groups: vec3<u32>) {{
	let {KERNEL_COUNT} = arrayLength(&data) - {outer}u - groups.x;
	var mapped = {};
	if (id.x < {KERNEL_COUNT}) {{ let {KERNEL_ITEM} = data[id.x]; mapped = {}; }}
	partial[local.x] = mapped;
	workgroupBarrier();
	for (var step = {}u; step > 0u; step = step / 2u) {{
		if (local.x < step) {{ partial[local.x] = {}; }}
		workgroupBarrier();
	}}
	if (local.x == 0u) {{ data[{KERNEL_COUNT} + {outer}u + group.x] = partial[0]; }}
}}", reduction.identity(), self.computed, WORKGROUP_SIZE / 2, reduction.combined("partial[local.x]", "partial[local.x + step]"))
	}
}

/// The kernel mapping every item of `data` by the lambda `parameter => body`, if WGSL can compute it
pub(crate) fn kernel(lambda: &Node) -> Option<Kernel> {
	let Node::Key(parameter, Op::FatArrow, body) = lambda.drop_meta() else { return None };
	let Node::Symbol(parameter) = parameter.drop_meta() else { return None };
	let mut translation = Translation { parameter, outer: vec![], heavy: false };
	let computed = translation.expression(body)?;
	let Translation { outer, heavy, .. } = translation;
	Some(Kernel { computed, outer, heavy })
}

/// The kernel of a lambda worth the GPU: one of only arithmetic is faster on the CPU at any count
pub(crate) fn gpu_kernel(lambda: &Node) -> Option<Kernel> {
	kernel(lambda).filter(|kernel| kernel.heavy)
}

fn is_light(lambda: &Node) -> bool {
	kernel(lambda).is_some_and(|kernel| !kernel.heavy)
}

struct Translation<'a> {
	parameter: &'a str,
	outer: Vec<String>,
	heavy: bool,
}

impl Translation<'_> {
	/// `body` in WGSL f32 arithmetic of the parameter, number literals and the program's numbers
	fn expression(&mut self, body: &Node) -> Option<String> {
		match body.drop_meta() {
			Node::Symbol(name) if name == self.parameter => Some(KERNEL_ITEM.to_string()),
			Node::Symbol(name) => {
				let index = self.outer.iter().position(|known| known == name).unwrap_or_else(|| {
					self.outer.push(name.clone());
					self.outer.len() - 1
				});
				Some(format!("data[{KERNEL_COUNT} + {index}u]"))
			}
			Node::Number(number) if !matches!(number, crate::extensions::numbers::Number::Complex(..)) => Some(float_literal(f64::from(*number))),
			Node::Key(empty, op, operand) if matches!(empty.drop_meta(), Node::Empty) => match op {
				Op::Neg => Some(format!("(-{})", self.expression(operand)?)),
				Op::Sqrt => Some(format!("sqrt({})", self.expression(operand)?)),
				Op::Abs => Some(format!("abs({})", self.expression(operand)?)),
				_ => None,
			},
			Node::Key(base, Op::Pow, exponent) => match exponent.drop_meta() {
				Node::Number(crate::extensions::numbers::Number::Int(power)) if (1..=LARGEST_MULTIPLIED_POWER).contains(power) => {
					let base = self.expression(base)?;
					Some(format!("({})", vec![base; *power as usize].join(" * ")))
				}
				_ => {
					self.heavy = true;
					Some(format!("pow({}, {})", self.expression(base)?, self.expression(exponent)?))
				}
			},
			Node::Key(left, op, right) => {
				let symbol = match op {
					Op::Add => "+",
					Op::Sub => "-",
					Op::Mul => "*",
					Op::Div => "/",
					_ => return None,
				};
				Some(format!("({} {symbol} {})", self.expression(left)?, self.expression(right)?))
			}
			Node::List(items, _, _) => match items.as_slice() {
				[single] => self.expression(single),
				[function, arguments @ ..] => {
					let name = function.name();
					let arguments = match arguments {
						// `max(x, 1)`: the arguments as one list
						[together] if KERNEL_FUNCTIONS_OF_TWO.contains(&name.as_str()) => match together.drop_meta() {
							Node::List(pair, _, _) => pair.as_slice(),
							_ => return None,
						},
						separate => separate,
					};
					let arity = if KERNEL_FUNCTIONS.contains(&name.as_str()) { 1 } else if KERNEL_FUNCTIONS_OF_TWO.contains(&name.as_str()) { 2 } else { 0 };
					if arity == 0 || arguments.len() != arity {
						return None;
					}
					self.heavy |= HEAVY_FUNCTIONS.contains(&name.as_str());
					let arguments = arguments.iter().map(|argument| self.expression(argument)).collect::<Option<Vec<_>>>()?;
					Some(format!("{name}({})", arguments.join(", ")))
				}
				_ => None,
			},
			_ => None,
		}
	}
}

/// A WGSL f32 literal: always with a point or an exponent, so `2` is not an i32
fn float_literal(number: f64) -> String {
	let written = format!("{:?}", number as f32);
	if written.contains(['.', 'e', 'E']) { written } else { format!("{written}.0") }
}

/// Every `@gpu` that shared_arrays will not lower to the GPU: a warning saying why it runs on the CPU (an error under
/// strict)
pub fn warn_unapplied(node: Node) -> Node {
	let mut warnings = vec![];
	collect_unapplied(&node, &mut warnings);
	match crate::diagnostic::report(&warnings) {
		Ok(()) => node,
		Err(error) => error,
	}
}

fn collect_unapplied(node: &Node, warnings: &mut Vec<crate::diagnostic::Diagnostic>) {
	let warn = |warnings: &mut Vec<_>, reason: String| warnings.push(crate::diagnostic::Diagnostic::at(node, format!("{reason}, so this runs on the CPU")));
	match node.drop_meta() {
		Node::Key(target, Op::Assign, value) if matches!(target.drop_meta(), Node::Symbol(_)) && (gpu_map(value).is_some() || gpu_reduction(value).is_some()) => {
			let map = gpu_reduction(value).map_or(value.as_ref().clone(), |(_, map)| map);
			if let Some(reason) = unapplied(&map) {
				warn(warnings, reason);
			}
		}
		_ if crate::parallel::annotated_here(node, GPU_ATTRIBUTE) => {
			let assign = if crate::parallel::map_call(node).is_some() { "@gpu maps into a new array or reduces to a number: assign it" } else { "@gpu runs only a map" };
			warn(warnings, format!("{assign}, `ys = xs.map(x => …) @gpu`, `s = sum(xs.map(x => …) @gpu)`"));
		}
		Node::Key(left, _, right) => [left, right].into_iter().for_each(|part| collect_unapplied(part, warnings)),
		Node::List(items, _, _) => items.iter().for_each(|item| collect_unapplied(item, warnings)),
		_ => {}
	}
}

/// Why the `@gpu` map `value` cannot run on the GPU, if it cannot
fn unapplied(value: &Node) -> Option<String> {
	let (_, lambda) = gpu_map(value)?;
	if is_light(&lambda) {
		return Some(format!("@gpu: the CPU maps `{}` faster: only arithmetic, ~1–5 ns an item (f64x2 lanes where it can), the GPU ~12", lambda.serialize().trim()));
	}
	kernel(&lambda).is_none().then(|| format!("@gpu: WGSL cannot compute `{}` (only + - * / ^ √ and math words of the item and numbers)", lambda.serialize().trim()))
}

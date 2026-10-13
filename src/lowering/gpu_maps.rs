//! `ys = xs.map(x => …) @gpu` (card gpu-vectors, P214: floats go to the GPU only where the program allows f32): a pure
//! numeric lambda over a linear float array runs as a WGSL compute kernel in f32. shared_arrays.rs lowers the map to
//! the host word gpu_map_linear, which reads xs's block and writes ys's (src/host.rs natively, host-gpu.js in the
//! browser); without an adapter it says so once and gives 0, and the CPU maps the items instead. Where `@gpu` cannot
//! apply, a warning says why and the map runs on the CPU as written (an error under strict). notes/gpu.md
//! Without @gpu (card gpu-auto, user 2026-10-10: GPU results are imprecise by design): a heavy map of a linear float
//! array goes to the GPU by itself from GPU_AUTO_MIN_COUNT items, saying so once at run time; `@cpu` on the map, its
//! block or its function keeps it on the CPU in f64, and WARP_GPU=off keeps a whole run there (@gpu included)

use crate::node::Node;
use crate::operators::Op;

const GPU_ATTRIBUTE: &str = "gpu";
/// `@cpu` on a map, a block or a function: never on the GPU, exact in f64
const CPU_ATTRIBUTE: &str = "cpu";
/// The mark of a map the compiler sends to the GPU without @gpu
const AUTOMATIC_ATTRIBUTE: &str = "gpu_automatic";
/// `WARP_GPU=off`: every map on the CPU, for test runs that compare exact values
const GPU_SWITCH: &str = "WARP_GPU";
const GPU_SWITCHED_OFF: &str = "off";
/// Threads per workgroup of the kernel; gpu_map_linear dispatches count / WORKGROUP_SIZE workgroups, rounded up
pub(crate) const WORKGROUP_SIZE: i64 = 256;
/// Fewer items map on the CPU (card gpu-threshold, notes/gpu.md): the GPU's ~2 ms of setup and readback loses below
/// ~3·10^4 items even against a heavy lambda (sin, cos: ~70–190 ns an item on the CPU, ~12 on the GPU)
pub(crate) const GPU_MAP_MIN_COUNT: i64 = 32768;
/// A map without @gpu goes to the GPU from 10× the break-even: an order of magnitude of room for slower GPUs and faster
/// CPUs, so the switch never makes a program slower (user 2026-10-10)
pub const GPU_AUTO_MIN_COUNT: i64 = 10 * GPU_MAP_MIN_COUNT;
/// The attributes marking an @gpu map whose result stays on the GPU for a later map of it, and that later map
const KEEP_RESULT_ATTRIBUTE: &str = "gpu_keep_result";
const SOURCE_KEPT_ATTRIBUTE: &str = "gpu_source_kept";
/// gpu_map_linear's and gpu_reduce_linear's last argument: keep the result's buffer on the GPU (under the target block's
/// address), take the source's items from such a buffer (src/graphics/gpu.rs KEPT; uploaded from memory when none is kept)
pub(crate) const KEEP_RESULT: i64 = 1;
pub(crate) const SOURCE_KEPT: i64 = 2;
/// The flag of a map the compiler switched to the GPU: no warning without an adapter, a notice when it ran there
pub(crate) const AUTOMATIC: i64 = 4;
/// What a switched map says once a run when it ran on the GPU (decision: GPU results are imprecise by design)
pub const AUTOMATIC_NOTICE: &str = "ran on the GPU by itself (f32, imprecise by design; write @cpu for exact f64)";
/// The lambda's parameter as the kernel names it (the user's name could be a WGSL keyword)
const KERNEL_ITEM: &str = "item";
/// The math words WGSL has under the same name and meaning (warp's log is the natural logarithm, as WGSL's)
const KERNEL_FUNCTIONS: [&str; 15] = [
	"sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "exp", "log", "sqrt", "abs", "floor", "ceil",
];
const KERNEL_FUNCTIONS_OF_TWO: [&str; 3] = ["min", "max", "atan2"];
/// The math words that cost the CPU tens of nanoseconds an item, so a lambda with one is worth the GPU
const HEAVY_FUNCTIONS: [&str; 12] = [
	"sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "exp", "log", "atan2",
];
/// The kernel's count of items: `data` holds them, then the program's numbers the lambda reads; a private variable,
/// which the program's functions the kernel calls read too
const KERNEL_COUNT: &str = "count";
/// The WGSL names of a program's function (card gpu-functions) and of its parameters and variables: prefixed, so a
/// warp name can be a WGSL keyword
const FUNCTION_PREFIX: &str = "warp_";
const VARIABLE_PREFIX: &str = "v_";
const PARAMETER_PREFIX: &str = "a_";
const RETURN_WORD: &str = "return";
const FOR_WORD: &str = "for";
const IN_WORD: &str = "in";
/// `x^k` for a whole k up to this is a product (pow of a negative base is NaN in WGSL, a number on the CPU)
const LARGEST_MULTIPLIED_POWER: i64 = 8;

/// `xs.map(f) @gpu` or `@gpu xs.map(f)`: xs and f; a chain `xs.map(f).map(g)` is xs and g after f, one kernel
pub(crate) fn gpu_map(value: &Node) -> Option<(Node, Node)> {
	let marked = crate::parallel::is_annotated(value, GPU_ATTRIBUTE) || value.attribute(AUTOMATIC_ATTRIBUTE).is_some();
	let (list, function) = (marked && !switched_off())
		.then(|| crate::parallel::map_call(value))
		.flatten()?;
	Some(fused(list, function))
}

fn switched_off() -> bool {
	std::env::var(GPU_SWITCH).is_ok_and(|switch| switch.eq_ignore_ascii_case(GPU_SWITCHED_OFF))
}

/// The minimum count for the GPU of a map with the flags `keeping`
pub(crate) fn fewest_items(keeping: i64) -> i64 {
	if keeping & AUTOMATIC != 0 {
		GPU_AUTO_MIN_COUNT
	} else {
		GPU_MAP_MIN_COUNT
	}
}

/// Every `ys = xs.map(f)` and `s = sum(xs.map(f))` (min, max) without @gpu, of a linear float array (`is_linear_floats`)
/// and a lambda worth the GPU, marked for it: the lowered map then checks GPU_AUTO_MIN_COUNT at run time. Not under
/// `@cpu`, on the map or on a block or function around it
pub(crate) fn automatic(node: Node, is_linear_floats: &impl Fn(&Node) -> bool) -> Node {
	automatic_in(node, is_linear_floats, switched_off())
}

fn automatic_in(node: Node, is_linear_floats: &impl Fn(&Node) -> bool, pinned: bool) -> Node {
	let pinned =
		pinned || crate::parallel::is_annotated(&node, CPU_ATTRIBUTE) || node.attribute(CPU_ATTRIBUTE).is_some();
	if pinned {
		return node;
	}
	let marked = match node.drop_meta() {
		Node::Key(target, Op::Assign, value) if matches!(target.drop_meta(), Node::Symbol(_)) => {
			automatic_value(value, is_linear_floats)
		}
		_ => None,
	};
	if let Some(marked) = marked {
		return with_value(node, marked);
	}
	node.map_children(|child| automatic_in(child, is_linear_floats, pinned))
}

/// The map, or the reduction's map, marked AUTOMATIC_ATTRIBUTE, when it qualifies
fn automatic_value(value: &Node, is_linear_floats: &impl Fn(&Node) -> bool) -> Option<Node> {
	let qualifies = |map: &Node| {
		!crate::parallel::is_annotated(map, GPU_ATTRIBUTE)
			&& !crate::parallel::is_annotated(map, CPU_ATTRIBUTE)
			&& crate::parallel::map_call(map)
				.map(|(list, function)| fused(list, function))
				.is_some_and(|(list, function)| is_linear_floats(&list) && gpu_kernel(&function).is_some())
	};
	if qualifies(value) {
		return Some(value.clone().with_attribute(AUTOMATIC_ATTRIBUTE, Node::True));
	}
	let Node::List(items, bracket, separator) = value.drop_meta() else {
		return None;
	};
	let [word, argument] = items.as_slice() else {
		return None;
	};
	REDUCTIONS
		.iter()
		.find(|reduction| word.drop_meta().name() == reduction.word())?;
	let (map, wrapped) = match argument.drop_meta() {
		Node::List(inner, crate::node::Bracket::Round, inner_separator) if inner.len() == 1 => {
			(&inner[0], Some(inner_separator.clone()))
		}
		_ => (argument, None),
	};
	if !qualifies(map) {
		return None;
	}
	let marked = map.clone().with_attribute(AUTOMATIC_ATTRIBUTE, Node::True);
	let argument = match wrapped {
		Some(inner_separator) => Node::List(vec![marked], crate::node::Bracket::Round, inner_separator),
		None => marked,
	};
	Some(Node::List(
		vec![word.clone(), argument],
		bracket.clone(),
		separator.clone(),
	))
}

/// The assignment with its value replaced
fn with_value(statement: Node, value: Node) -> Node {
	match statement {
		Node::Meta { node, data } => Node::Meta {
			node: Box::new(with_value(*node, value)),
			data,
		},
		Node::Key(target, op, _) => Node::Key(target, op, Box::new(value)),
		other => other,
	}
}

/// The map of `list` by `function` with the maps `list` itself is made of folded in: `xs.map(f).map(g)` is xs and g
/// after f, one pass over xs without a list of f's results
pub(crate) fn fused(mut list: Node, mut function: Node) -> (Node, Node) {
	loop {
		let Some((inner_list, inner_function)) = crate::parallel::map_call(&list) else {
			return (list, function);
		};
		let Some(both) = composed(&inner_function, &function) else {
			return (list, function);
		};
		(list, function) = (inner_list, both);
	}
}

/// `x => g_body` after `x => f_body`: `x => g_body` with g's parameter replaced by f's body (none when g reads a name
/// f's parameter would capture)
fn composed(first: &Node, then: &Node) -> Option<Node> {
	let (Node::Key(parameter, Op::FatArrow, body), Node::Key(then_parameter, Op::FatArrow, then_body)) =
		(first.drop_meta(), then.drop_meta())
	else {
		return None;
	};
	let (Node::Symbol(name), Node::Symbol(then_name)) = (parameter.drop_meta(), then_parameter.drop_meta()) else {
		return None;
	};
	if name != then_name && crate::warp_parser::mentions(then_body, name) {
		return None;
	}
	let grouped = Node::List(
		vec![body.as_ref().clone()],
		crate::node::Bracket::Round,
		crate::node::Separator::None,
	);
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
	let Node::List(
		mut statements,
		bracket,
		separator @ (crate::node::Separator::Semicolon | crate::node::Separator::Newline),
	) = node
	else {
		return node;
	};
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
		let Some((name, value)) = gpu_assignment(&statements[index]) else {
			continue;
		};
		// a map of only arithmetic runs on the CPU and leaves no buffer on the GPU
		if gpu_map(&value)
			.and_then(|(_, function)| gpu_kernel(&function))
			.is_none()
		{
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
	let Node::Key(target, Op::Assign, value) = statement.drop_meta() else {
		return false;
	};
	let map = match gpu_reduction(value) {
		Some((_, map)) => map,
		None => value.as_ref().clone(),
	};
	matches!(target.drop_meta(), Node::Symbol(_))
		&& gpu_map(&map).is_some_and(|(list, _)| matches!(list.drop_meta(), Node::Symbol(source) if source == name))
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
		Node::Meta { node, data } => Node::Meta {
			node: Box::new(with_attribute_on_value(*node, attribute)),
			data,
		},
		Node::Key(target, op, value) => Node::Key(target, op, Box::new(value.with_attribute(attribute, Node::True))),
		other => other,
	}
}

/// The flags of an @gpu map's value for the host word: KEEP_RESULT, SOURCE_KEPT
pub(crate) fn keeping(value: &Node) -> i64 {
	[
		(KEEP_RESULT_ATTRIBUTE, KEEP_RESULT),
		(SOURCE_KEPT_ATTRIBUTE, SOURCE_KEPT),
		(AUTOMATIC_ATTRIBUTE, AUTOMATIC),
	]
	.iter()
	.filter(|(attribute, _)| value.attribute(attribute).is_some())
	.map(|(_, flag)| flag)
	.sum()
}

/// The statements with statement `index`, an @gpu map whose result only later @gpu maps read, folded into those
fn folded_into_consumers(statements: &[Node], index: usize, program: &Node) -> Option<Vec<Node>> {
	let (name, value) = gpu_assignment(&statements[index])?;
	let later = &statements[index + 1..];
	let mut read = vec![];
	value.drop_meta().visit(&mut |part| {
		if let Node::Symbol(symbol) = part {
			read.push(symbol.clone())
		}
	});
	if later
		.iter()
		.any(|statement| read.iter().any(|symbol| writes(statement, symbol)))
	{
		return None;
	}
	let uses = uses_of(program, &name);
	let bare = value.drop_meta().clone();
	let mut consumers = 0;
	let rewritten: Vec<Node> = later
		.iter()
		.map(|statement| match gpu_assignment(statement) {
			Some((_, consumer))
				if crate::parallel::map_call(&consumer)
					.is_some_and(|(list, _)| matches!(list.drop_meta(), Node::Symbol(source) if *source == name)) =>
			{
				consumers += 1;
				crate::library_words::substitute(statement.clone(), &name, &bare)
			}
			_ => statement.clone(),
		})
		.collect();
	let all_fold = rewritten
		.iter()
		.zip(later)
		.filter(|(new, old)| new != old)
		.all(|(new, _)| {
			gpu_assignment(new)
				.and_then(|(_, value)| gpu_map(&value))
				.is_some_and(|(list, _)| crate::parallel::map_call(&list).is_none())
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
	let Node::Key(target, Op::Assign, value) = statement.drop_meta() else {
		return None;
	};
	let Node::Symbol(name) = target.drop_meta() else {
		return None;
	};
	gpu_map(value).map(|_| (name.clone(), value.as_ref().clone()))
}

/// Whether `node` assigns `name` or one of its items
fn writes(node: &Node, name: &str) -> bool {
	let mut found = false;
	node.visit(&mut |part| {
		if let Node::Key(target, op, _) = part {
			let assigned = match target.drop_meta() {
				Node::Key(list, Op::Hash, _) => list.drop_meta(),
				other => other,
			};
			found |= (*op == Op::Assign || op.is_compound_assign())
				&& matches!(assigned, Node::Symbol(symbol) if symbol == name);
		}
	});
	found
}

/// A lambda as a WGSL compute kernel: the item it computes in WGSL, the program's numbers it reads (`outer`, appended
/// to `data` after the items, in this order) and whether it is worth the GPU (math words, powers: `heavy`)
pub(crate) struct Kernel {
	computed: String,
	pub outer: Vec<String>,
	heavy: bool,
	/// The program's functions it calls, in WGSL
	functions: String,
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
	let Node::List(items, _, _) = value.drop_meta() else {
		return None;
	};
	let [word, argument] = items.as_slice() else {
		return None;
	};
	let reduction = REDUCTIONS
		.into_iter()
		.find(|reduction| word.drop_meta().name() == reduction.word())?;
	let argument = match argument.drop_meta() {
		Node::List(inner, crate::node::Bracket::Round, _) if inner.len() == 1 => &inner[0],
		_ => argument,
	};
	gpu_map(argument).map(|_| (reduction, argument.clone()))
}

const KERNEL_BINDING: &str = "@group(0) @binding(0) var<storage, read_write> data: array<f32>;
var<private> count: u32;";

impl Kernel {
	/// The shader mapping every item of `data` in place
	pub(crate) fn map_shader(&self) -> String {
		format!(
			"{KERNEL_BINDING}
@compute @workgroup_size({WORKGROUP_SIZE}) fn main(@builtin(global_invocation_id) id: vec3<u32>) {{
	{KERNEL_COUNT} = arrayLength(&data) - {}u;
	if (id.x < {KERNEL_COUNT}) {{ let {KERNEL_ITEM} = data[id.x]; data[id.x] = {}; }}
}}{}",
			self.outer.len(),
			self.computed,
			self.functions
		)
	}

	/// The shader reducing the mapped items of each workgroup in its shared memory, halving the active threads each
	/// step, into one cell per workgroup after the items and the outer numbers
	pub(crate) fn reduce_shader(&self, reduction: Reduction) -> String {
		let outer = self.outer.len();
		format!("{KERNEL_BINDING}
var<workgroup> partial: array<f32, {WORKGROUP_SIZE}>;
@compute @workgroup_size({WORKGROUP_SIZE}) fn main(@builtin(global_invocation_id) id: vec3<u32>, @builtin(local_invocation_id) local: vec3<u32>, @builtin(workgroup_id) group: vec3<u32>, @builtin(num_workgroups) groups: vec3<u32>) {{
	{KERNEL_COUNT} = arrayLength(&data) - {outer}u - groups.x;
	var mapped = {};
	if (id.x < {KERNEL_COUNT}) {{ let {KERNEL_ITEM} = data[id.x]; mapped = {}; }}
	partial[local.x] = mapped;
	workgroupBarrier();
	for (var step = {}u; step > 0u; step = step / 2u) {{
		if (local.x < step) {{ partial[local.x] = {}; }}
		workgroupBarrier();
	}}
	if (local.x == 0u) {{ data[{KERNEL_COUNT} + {outer}u + group.x] = partial[0]; }}
}}{}", reduction.identity(), self.computed, WORKGROUP_SIZE / 2, reduction.combined("partial[local.x]", "partial[local.x + step]"), self.functions)
	}
}

/// The kernel mapping every item of `data` by the lambda `parameter => body`, if WGSL can compute it
pub(crate) fn kernel(lambda: &Node) -> Option<Kernel> {
	let Node::Key(parameter, Op::FatArrow, body) = lambda.drop_meta() else {
		return None;
	};
	let Node::Symbol(parameter) = parameter.drop_meta() else {
		return None;
	};
	let mut translation = Translation {
		parameter: parameter.clone(),
		..Translation::default()
	};
	let computed = translation.expression(body)?;
	let Translation {
		outer,
		heavy,
		functions,
		..
	} = translation;
	Some(Kernel {
		computed,
		outer,
		heavy,
		functions: functions.into_iter().map(|function| format!("\n{function}")).collect(),
	})
}

/// The kernel of a lambda the CPU maps as one loop over linear memory: without calls of the program's functions, which
/// go to the GPU only (card gpu-functions), the CPU calling them on a list as before
pub(crate) fn linear_kernel(lambda: &Node) -> Option<Kernel> {
	kernel(lambda).filter(|kernel| kernel.functions.is_empty())
}

/// The kernel of a lambda worth the GPU: one of only arithmetic is faster on the CPU at any count
pub(crate) fn gpu_kernel(lambda: &Node) -> Option<Kernel> {
	kernel(lambda).filter(|kernel| kernel.heavy)
}

fn is_light(lambda: &Node) -> bool {
	kernel(lambda).is_some_and(|kernel| !kernel.heavy)
}

/// A program's function a kernel may call: its parameters and body
#[derive(Clone)]
struct ProgramFunction {
	parameters: Vec<String>,
	body: Node,
}

thread_local! {
	/// The functions of the program being lowered, by name (card gpu-functions)
	static PROGRAM_FUNCTIONS: std::cell::RefCell<std::collections::HashMap<String, ProgramFunction>> =
		Default::default();
}

/// `run` knowing the functions `program` defines, so a kernel may call them
pub(crate) fn with_program_functions<T>(program: &Node, run: impl FnOnce() -> T) -> T {
	let statements = match program.drop_meta() {
		Node::List(items, _, _) => items.as_slice(),
		single => std::slice::from_ref(single),
	};
	let functions = statements
		.iter()
		.filter_map(crate::law::function_definition)
		.map(|definition| {
			let parameters = definition.parameters.into_iter().map(|(name, _)| name).collect();
			(
				definition.name,
				ProgramFunction {
					parameters,
					body: definition.body,
				},
			)
		})
		.collect();
	let before = PROGRAM_FUNCTIONS.with(|known| known.replace(functions));
	let result = run();
	PROGRAM_FUNCTIONS.with(|known| known.replace(before));
	result
}

fn program_function(name: &str) -> Option<ProgramFunction> {
	PROGRAM_FUNCTIONS.with(|known| known.borrow().get(name).cloned())
}

#[derive(Default)]
struct Translation {
	/// The lambda's parameter, the kernel's item; none inside a function
	parameter: String,
	outer: Vec<String>,
	heavy: bool,
	/// The parameters and variables of the function being translated, WGSL variables
	locals: Vec<String>,
	/// The program's functions translated so far, in WGSL, callees first
	functions: Vec<String>,
	translated: Vec<String>,
	/// The functions being translated: a call of one of them is recursion, which WGSL forbids
	in_progress: Vec<String>,
}

impl Translation {
	/// `body` in WGSL f32 arithmetic of the parameter, number literals and the program's numbers
	fn expression(&mut self, body: &Node) -> Option<String> {
		match body.drop_meta() {
			Node::Symbol(name) if self.locals.contains(name) => Some(format!("{VARIABLE_PREFIX}{name}")),
			Node::Symbol(name) if *name == self.parameter => Some(KERNEL_ITEM.to_string()),
			Node::Symbol(name) => {
				let index = self.outer.iter().position(|known| known == name).unwrap_or_else(|| {
					self.outer.push(name.clone());
					self.outer.len() - 1
				});
				Some(format!("data[{KERNEL_COUNT} + {index}u]"))
			}
			Node::Number(number) if !matches!(number, crate::extensions::numbers::Number::Complex(..)) => {
				Some(float_literal(f64::from(*number)))
			}
			Node::Key(empty, op, operand) if matches!(empty.drop_meta(), Node::Empty) => match op {
				Op::Neg => Some(format!("(-{})", self.expression(operand)?)),
				Op::Sqrt => Some(format!("sqrt({})", self.expression(operand)?)),
				Op::Abs => Some(format!("abs({})", self.expression(operand)?)),
				Op::Not => Some(format!("select(0.0, 1.0, !{})", self.condition(operand)?)),
				_ => None,
			},
			Node::Key(base, Op::Pow, exponent) => match exponent.drop_meta() {
				Node::Number(crate::extensions::numbers::Number::Int(power))
					if (1..=LARGEST_MULTIPLIED_POWER).contains(power) =>
				{
					let base = self.expression(base)?;
					Some(format!("({})", vec![base; *power as usize].join(" * ")))
				}
				_ => {
					self.heavy = true;
					Some(format!(
						"pow({}, {})",
						self.expression(base)?,
						self.expression(exponent)?
					))
				}
			},
			Node::Key(condition, Op::Question, branches) => match branches.drop_meta() {
				Node::Key(then, Op::Colon, otherwise) => self.selected(condition, then, otherwise),
				_ => None,
			},
			Node::Key(if_then, Op::Else, otherwise) => {
				let (condition, then) = if_then_parts(if_then)?;
				self.selected(condition, then, otherwise)
			}
			Node::Key(_, op, _) if comparison(*op).is_some() || matches!(op, Op::And | Op::Or) => {
				Some(format!("select(0.0, 1.0, {})", self.condition(body)?))
			}
			Node::Key(left, op, right) => {
				let symbol = match op {
					Op::Add => "+",
					Op::Sub => "-",
					Op::Mul => "*",
					Op::Div => "/",
					_ => return None,
				};
				Some(format!(
					"({} {symbol} {})",
					self.expression(left)?,
					self.expression(right)?
				))
			}
			Node::List(items, _, _) => match items.as_slice() {
				[single] => self.expression(single),
				[function, arguments @ ..] => {
					let name = function.name();
					if let Some(called) = program_function(&name) {
						return self.call(&name, &called, arguments);
					}
					let arguments = match arguments {
						// `max(x, 1)`: the arguments as one list
						[together] if KERNEL_FUNCTIONS_OF_TWO.contains(&name.as_str()) => match together.drop_meta() {
							Node::List(pair, _, _) => pair.as_slice(),
							_ => return None,
						},
						separate => separate,
					};
					let arity = if KERNEL_FUNCTIONS.contains(&name.as_str()) {
						1
					} else if KERNEL_FUNCTIONS_OF_TWO.contains(&name.as_str()) {
						2
					} else {
						0
					};
					if arity == 0 || arguments.len() != arity {
						return None;
					}
					self.heavy |= HEAVY_FUNCTIONS.contains(&name.as_str());
					let arguments = self.expressions(arguments)?;
					Some(format!("{name}({})", arguments.join(", ")))
				}
				_ => None,
			},
			_ => None,
		}
	}

	fn expressions(&mut self, nodes: &[Node]) -> Option<Vec<String>> {
		nodes.iter().map(|node| self.expression(node)).collect()
	}

	/// `condition ? then : otherwise` in an expression
	fn selected(&mut self, condition: &Node, then: &Node, otherwise: &Node) -> Option<String> {
		let condition = self.condition(condition)?;
		Some(format!(
			"select({}, {}, {condition})",
			self.expression(otherwise)?,
			self.expression(then)?
		))
	}

	/// A condition as a WGSL bool: comparisons, and, or, not; a number is true unless 0
	fn condition(&mut self, node: &Node) -> Option<String> {
		match node.drop_meta() {
			Node::Key(empty, Op::Not, operand) if matches!(empty.drop_meta(), Node::Empty) => {
				Some(format!("!{}", self.condition(operand)?))
			}
			Node::Key(left, op @ (Op::And | Op::Or), right) => {
				let symbol = if *op == Op::And { "&&" } else { "||" };
				Some(format!(
					"({} {symbol} {})",
					self.condition(left)?,
					self.condition(right)?
				))
			}
			Node::Key(left, op, right) if comparison(*op).is_some() => Some(format!(
				"({} {} {})",
				self.expression(left)?,
				comparison(*op)?,
				self.expression(right)?
			)),
			Node::List(items, _, _) if items.len() == 1 => self.condition(&items[0]),
			_ => Some(format!("({} != 0.0)", self.expression(node)?)),
		}
	}

	/// A call of the program's function `name`: translated once into a WGSL function; none for recursion
	fn call(&mut self, name: &str, called: &ProgramFunction, arguments: &[Node]) -> Option<String> {
		let arguments = match arguments {
			// `f(a, b)`: the arguments as one list
			[together] if called.parameters.len() > 1 => match together.drop_meta() {
				Node::List(items, _, _) => items.as_slice(),
				_ => return None,
			},
			separate => separate,
		};
		if arguments.len() != called.parameters.len() || self.in_progress.iter().any(|caller| caller == name) {
			return None;
		}
		let arguments = self.expressions(arguments)?;
		if !self.translated.iter().any(|done| done == name) {
			self.in_progress.push(name.to_string());
			let function = self.function(name, called);
			self.in_progress.pop();
			self.functions.push(function?);
			self.translated.push(name.to_string());
		}
		Some(format!("{FUNCTION_PREFIX}{name}({})", arguments.join(", ")))
	}

	/// The WGSL function of the program's function `name`: its parameters copied into variables, every variable it
	/// assigns declared first, its statements, the last one's value returned
	fn function(&mut self, name: &str, called: &ProgramFunction) -> Option<String> {
		let mut variables = called.parameters.clone();
		assigned_names(&called.body, &mut variables);
		let declared: String = variables
			.iter()
			.map(|variable| match called.parameters.contains(variable) {
				true => format!("\tvar {VARIABLE_PREFIX}{variable} = {PARAMETER_PREFIX}{variable};\n"),
				false => format!("\tvar {VARIABLE_PREFIX}{variable}: f32 = 0.0;\n"),
			})
			.collect();
		let caller = (
			std::mem::take(&mut self.parameter),
			std::mem::replace(&mut self.locals, variables),
		);
		let mut body = String::new();
		let returned = self.returned(&called.body, &mut body);
		(self.parameter, self.locals) = caller;
		returned?;
		let parameters: Vec<String> = called
			.parameters
			.iter()
			.map(|parameter| format!("{PARAMETER_PREFIX}{parameter}: f32"))
			.collect();
		Some(format!(
			"fn {FUNCTION_PREFIX}{name}({}) -> f32 {{\n{declared}{body}}}",
			parameters.join(", ")
		))
	}

	/// The statements of a function's body into `out`, every path ending in a return of its last value
	fn returned(&mut self, node: &Node, out: &mut String) -> Option<()> {
		if let Some(statements) = block_statements(node) {
			let (last, before) = statements.split_last()?;
			for statement in before {
				self.statement(statement, out)?;
			}
			return self.returned(last, out);
		}
		match node.drop_meta() {
			Node::Key(if_then, Op::Else, otherwise) => {
				let (condition, then) = if_then_parts(if_then)?;
				*out += &format!("if {} {{\n", self.condition(condition)?);
				self.returned(then, out)?;
				*out += "} else {\n";
				self.returned(otherwise, out)?;
				*out += "}\n";
				Some(())
			}
			Node::Key(target, op, _) if is_assignment(*op) && matches!(target.drop_meta(), Node::Symbol(_)) => {
				self.statement(node, out)?;
				*out += &format!("return {VARIABLE_PREFIX}{};\n", target.name());
				Some(())
			}
			_ if returned_value(node).is_some() => self.statement(node, out),
			_ if is_statement_only(node) => None,
			_ => {
				*out += &format!("return {};\n", self.expression(node)?);
				Some(())
			}
		}
	}

	/// One statement of a function in WGSL into `out`: an assignment, if, while, `for i in a..b`, return, a block
	fn statement(&mut self, node: &Node, out: &mut String) -> Option<()> {
		if let Some(statements) = block_statements(node) {
			return statements
				.iter()
				.try_for_each(|statement| self.statement(statement, out));
		}
		if let Some(value) = returned_value(node) {
			*out += &format!("return {};\n", self.expression(value)?);
			return Some(());
		}
		if let Some((variable, from, op, to, body)) = range_loop(node) {
			self.heavy = true;
			let (variable, from, to) = (
				format!("{VARIABLE_PREFIX}{variable}"),
				self.expression(from)?,
				self.expression(to)?,
			);
			let below = if op == Op::To { "<=" } else { "<" };
			*out += &format!("for ({variable} = {from}; {variable} {below} {to}; {variable} = {variable} + 1.0) {{\n");
			self.statement(body, out)?;
			*out += "}\n";
			return Some(());
		}
		match node.drop_meta() {
			Node::Key(target, op, value) if is_assignment(*op) => {
				let Node::Symbol(name) = target.drop_meta() else {
					return None;
				};
				let value = match op {
					Op::Assign | Op::Define => self.expression(value)?,
					compound => self.expression(&Node::Key(target.clone(), compound.base_op(), value.clone()))?,
				};
				*out += &format!("{VARIABLE_PREFIX}{name} = {value};\n");
				Some(())
			}
			Node::Key(if_then, Op::Else, otherwise) => {
				let (condition, then) = if_then_parts(if_then)?;
				*out += &format!("if {} {{\n", self.condition(condition)?);
				self.statement(then, out)?;
				*out += "} else {\n";
				self.statement(otherwise, out)?;
				*out += "}\n";
				Some(())
			}
			Node::Key(_, Op::Then, _) => {
				let (condition, then) = if_then_parts(node)?;
				*out += &format!("if {} {{\n", self.condition(condition)?);
				self.statement(then, out)?;
				*out += "}\n";
				Some(())
			}
			Node::Key(condition, Op::Do, body) => {
				let Node::Key(empty, Op::While, condition) = condition.drop_meta() else {
					return None;
				};
				if !matches!(empty.drop_meta(), Node::Empty) {
					return None;
				}
				self.heavy = true;
				*out += &format!("while {} {{\n", self.condition(condition)?);
				self.statement(body, out)?;
				*out += "}\n";
				Some(())
			}
			// a value nothing uses: checked, then dropped
			_ => self.expression(node).map(|_| ()),
		}
	}
}

/// The WGSL comparison of a warp operator
fn comparison(op: Op) -> Option<&'static str> {
	Some(match op {
		Op::Lt => "<",
		Op::Gt => ">",
		Op::Le => "<=",
		Op::Ge => ">=",
		Op::Eq => "==",
		Op::Ne => "!=",
		_ => return None,
	})
}

fn is_assignment(op: Op) -> bool {
	matches!(op, Op::Assign | Op::Define) || op.is_compound_assign()
}

/// `if c then t` (`if c { t }`): c and t
fn if_then_parts(node: &Node) -> Option<(&Node, &Node)> {
	let Node::Key(condition, Op::Then, then) = node.drop_meta() else {
		return None;
	};
	match condition.drop_meta() {
		Node::Key(empty, Op::If, condition) if matches!(empty.drop_meta(), Node::Empty) => Some((condition, then)),
		_ => None,
	}
}

/// The statements of a block: `{ … }`, or lines and `;` without brackets
fn block_statements(node: &Node) -> Option<&[Node]> {
	match node.drop_meta() {
		Node::List(items, crate::node::Bracket::Curly, _) => Some(items.as_slice()),
		Node::List(
			items,
			crate::node::Bracket::None,
			crate::node::Separator::Semicolon | crate::node::Separator::Newline,
		) => Some(items.as_slice()),
		_ => None,
	}
}

/// `return v`: v
fn returned_value(node: &Node) -> Option<&Node> {
	match node.drop_meta() {
		Node::List(items, _, _) if items.len() == 2 && items[0].is_symbol(RETURN_WORD) => Some(&items[1]),
		_ => None,
	}
}

/// `for i in a..b { … }` (`a to b` includes b): i, a, the range operator, b and the body
fn range_loop(node: &Node) -> Option<(String, &Node, Op, &Node, &Node)> {
	let Node::List(items, _, _) = node.drop_meta() else {
		return None;
	};
	let [for_word, variable, in_word, range, body] = items.as_slice() else {
		return None;
	};
	if !for_word.is_symbol(FOR_WORD) || !in_word.is_symbol(IN_WORD) {
		return None;
	}
	let Node::Symbol(variable) = variable.drop_meta() else {
		return None;
	};
	match range.drop_meta() {
		Node::Key(from, op @ (Op::Range | Op::To), to) => Some((variable.clone(), from, *op, to, body)),
		_ => None,
	}
}

/// A loop or `if` without else: no value to return, so a map calling a function ending in one stays on the CPU
fn is_statement_only(node: &Node) -> bool {
	range_loop(node).is_some() || matches!(node.drop_meta(), Node::Key(_, Op::Then | Op::Do, _))
}

/// Every name a function body assigns or loops over, once, after `names`
fn assigned_names(node: &Node, names: &mut Vec<String>) {
	let mut add = |name: String| {
		if !names.contains(&name) {
			names.push(name);
		}
	};
	node.visit(&mut |part| match part.drop_meta() {
		Node::Key(target, op, _) if is_assignment(*op) => {
			if let Node::Symbol(name) = target.drop_meta() {
				add(name.clone());
			}
		}
		_ => {
			if let Some((variable, ..)) = range_loop(part) {
				add(variable);
			}
		}
	});
}

/// A WGSL f32 literal: always with a point or an exponent, so `2` is not an i32
fn float_literal(number: f64) -> String {
	let written = format!("{:?}", number as f32);
	if written.contains(['.', 'e', 'E']) {
		written
	} else {
		format!("{written}.0")
	}
}

/// Every `@gpu` that shared_arrays will not lower to the GPU: a warning saying why it runs on the CPU (an error under
/// strict)
pub fn warn_unapplied(node: Node) -> Node {
	if let Some(both) = both_gpu_and_cpu(&node) {
		return crate::diagnostic::Diagnostic::at(
			&both,
			"@gpu and @cpu on the same map: keep one (@cpu: exact, on the CPU; @gpu: f32, on the GPU)",
		)
		.into_error();
	}
	let mut warnings = vec![];
	collect_unapplied(&node, &mut warnings);
	match crate::diagnostic::report(&warnings) {
		Ok(()) => node,
		Err(error) => error,
	}
}

/// The first form marked both @gpu and @cpu
fn both_gpu_and_cpu(node: &Node) -> Option<Node> {
	if crate::parallel::is_annotated(node, GPU_ATTRIBUTE) && crate::parallel::is_annotated(node, CPU_ATTRIBUTE) {
		return Some(node.clone());
	}
	match node {
		Node::Meta { node, .. } => both_gpu_and_cpu(node),
		Node::Key(left, _, right) => both_gpu_and_cpu(left).or_else(|| both_gpu_and_cpu(right)),
		Node::List(items, _, _) => items.iter().find_map(both_gpu_and_cpu),
		_ => None,
	}
}

fn collect_unapplied(node: &Node, warnings: &mut Vec<crate::diagnostic::Diagnostic>) {
	let warn = |warnings: &mut Vec<_>, reason: String| {
		warnings.push(crate::diagnostic::Diagnostic::at(node, format!("{reason}, so this runs on the CPU")).weak())
	};
	match node.drop_meta() {
		Node::Key(target, Op::Assign, value)
			if matches!(target.drop_meta(), Node::Symbol(_))
				&& (gpu_map(value).is_some() || gpu_reduction(value).is_some()) =>
		{
			let map = gpu_reduction(value).map_or(value.as_ref().clone(), |(_, map)| map);
			if let Some(reason) = unapplied(&map) {
				warn(warnings, reason);
			}
		}
		_ if crate::parallel::annotated_here(node, GPU_ATTRIBUTE) => {
			let assign = if crate::parallel::map_call(node).is_some() {
				"@gpu maps into a new array or reduces to a number: assign it"
			} else {
				"@gpu runs only a map"
			};
			warn(
				warnings,
				format!("{assign}, `ys = xs.map(x => …) @gpu`, `s = sum(xs.map(x => …) @gpu)`"),
			);
		}
		Node::Key(left, _, right) => [left, right]
			.into_iter()
			.for_each(|part| collect_unapplied(part, warnings)),
		Node::List(items, _, _) => items.iter().for_each(|item| collect_unapplied(item, warnings)),
		_ => {}
	}
}

/// Why the `@gpu` map `value` cannot run on the GPU, if it cannot
fn unapplied(value: &Node) -> Option<String> {
	let (_, lambda) = gpu_map(value)?;
	if is_light(&lambda) {
		return Some(format!(
			"@gpu: the CPU maps `{}` faster: only arithmetic, ~1–5 ns an item (f64x2 lanes where it can), the GPU ~12",
			lambda.serialize().trim()
		));
	}
	kernel(&lambda).is_none().then(|| {
		format!(
			"@gpu: WGSL cannot compute `{}` (only + - * / ^ √ and math words of the item and numbers)",
			lambda.serialize().trim()
		)
	})
}

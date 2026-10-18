//! WGSL from warp: a numeric lambda as a compute kernel (with its reductions), a painted function as a fragment shader
use super::*;
use crate::lowering::nodes::{is_binding, is_return};
use std::collections::{HashMap, HashSet};
use warp_runtime::host_words::GPU_PAINTS;

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

pub(super) const REDUCTIONS: [Reduction; 3] = [Reduction::Sum, Reduction::Min, Reduction::Max];
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
	let reduction = REDUCTIONS.into_iter().find(|reduction| word.drop_meta().name() == reduction.word())?;
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
	let Node::Key(parameter, Op::Arrow, body) = lambda.drop_meta() else {
		return None;
	};
	let parameter = parameter.symbol_name()?;
	let mut translation = Translation { parameter: parameter.to_string(), ..Translation::default() };
	let computed = translation.expression(body)?;
	let Translation { outer, heavy, functions, .. } = translation;
	Some(Kernel {
		computed,
		outer,
		heavy,
		functions: functions.into_iter().map(|function| format!("\n{function}")).collect(),
	})
}

/// Why WGSL cannot compute the lambda: the first construct it cannot, with its place (card kernel-refusal-reason)
pub(crate) fn kernel_refusal(lambda: &Node) -> Option<String> {
	let Node::Key(parameter, Op::Arrow, body) = lambda.drop_meta() else {
		return None;
	};
	let mut translation = Translation { parameter: parameter.symbol_name()?.to_string(), ..Translation::default() };
	translation.expression(body).is_none().then_some(translation.refusal).flatten()
}

/// The kernel of a lambda mapping a linear array: one calling the program's functions only when heavy, for the GPU
/// (card gpu-functions); a light one stays a list the CPU maps by calling them, as before
pub(crate) fn linear_kernel(lambda: &Node) -> Option<Kernel> {
	kernel(lambda).filter(|kernel| kernel.functions.is_empty() || kernel.heavy)
}

/// The kernel of a lambda worth the GPU: one of only arithmetic is faster on the CPU at any count
pub(crate) fn gpu_kernel(lambda: &Node) -> Option<Kernel> {
	kernel(lambda).filter(|kernel| kernel.heavy)
}

pub(super) fn is_light(lambda: &Node) -> bool {
	kernel(lambda).is_some_and(|kernel| !kernel.heavy)
}

/// A program's function a kernel may call: its parameters and body
#[derive(Clone)]
pub(crate) struct ProgramFunction {
	parameters: Vec<String>,
	body: Node,
}

/// `run` knowing the functions `program` defines, so a kernel may call them
pub(crate) fn with_program_functions<T>(program: &Node, run: impl FnOnce() -> T) -> T {
	let statements = match program.drop_meta() {
		Node::List(items, _, _) => items.as_slice(),
		single => std::slice::from_ref(single),
	};
	let classes = crate::analyzer::declared_classes(program);
	let functions = statements
		.iter()
		.filter_map(crate::law::function_definition)
		.map(|definition| {
			let parameters = definition.parameters.into_iter().map(|(name, _)| name).collect();
			let body = without_instance_dispatch(definition.body, &classes);
			(definition.name, ProgramFunction { parameters, body })
		})
		.collect();
	let mut values = HashMap::new();
	for (name, value) in statements.iter().filter_map(main_level_assignment) {
		values.entry(name.to_string()).or_insert_with(|| value.clone());
	}
	crate::compiler::compile_context::scoped(
		|context| &mut context.program_values,
		values,
		|| crate::compiler::compile_context::scoped(|context| &mut context.program_functions, functions, run),
	)
}

/// `name = value`, `name: float = value`, `global name = value`: the variable and the value the main level assigns it
fn main_level_assignment(statement: &Node) -> Option<(&str, &Node)> {
	let statement = match statement.drop_meta() {
		Node::Key(keyword, Op::Colon, declared) if keyword.is_symbol(crate::words::GLOBAL_WORD) => declared.drop_meta(),
		other => other,
	};
	let Node::Key(target, Op::Assign, value) = statement else {
		return None;
	};
	let name = match target.drop_meta() {
		Node::Key(name, Op::Colon, _) => name.symbol_name()?,
		other => other.symbol_name()?,
	};
	Some((name, value))
}

fn program_value(name: &str) -> Option<Node> {
	crate::compiler::compile_context::read(|context| context.program_values.get(name).cloned())
}

/// `x + 1` of a program with a class defining plus (`use sound`: Note) reads `if x is Note then x.plus(1) else x + 1`
/// (class_methods.rs): on the GPU no value is an instance, so only the built-in operator remains
fn without_instance_dispatch(node: Node, classes: &HashSet<String>) -> Node {
	let otherwise = crate::lowering::nodes::branches(&node).and_then(|(condition, then, otherwise)| {
		let Node::Key(subject, Op::Eq, class) = condition.drop_meta() else { return None };
		let Node::Key(receiver, Op::Dot, _) = then.drop_meta() else { return None };
		let is_dispatch = class.symbol_name().is_some_and(|class| classes.contains(class)) && receiver == subject;
		is_dispatch.then(|| otherwise.clone())
	});
	match otherwise {
		Some(otherwise) => without_instance_dispatch(otherwise, classes),
		None => node.map_children(|child| without_instance_dispatch(child, classes)),
	}
}

fn program_function(name: &str) -> Option<ProgramFunction> {
	crate::compiler::compile_context::read(|context| context.program_functions.get(name).cloned())
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
	/// Their WGSL names with their result's width
	translated: Vec<(String, usize)>,
	/// The functions being translated: a call of one of them is recursion, which WGSL forbids
	in_progress: Vec<String>,
	/// The program's numbers are a fragment shader's `values` (paint), not cells of the kernel's `data`
	uniforms: bool,
	/// The program's lists the shader reads item by item, `levels#k`: its values hold one vec4f per item
	item_lists: Vec<String>,
	/// The function being translated gives a color, a vec4f
	color_result: bool,
	/// The widths of the function's vectors among its locals (card compile-shaders); a local not here is an f32
	widths: HashMap<String, usize>,
	/// The width every return of the function being translated gives
	returned_width: Option<usize>,
	/// The first construct WGSL cannot compute, with its place: the reason shader_of, paint and @gpu name
	/// (card kernel-refusal-reason)
	refusal: Option<String>,
	/// The program's variables whose width is being found from their values: one naming itself has none
	resolving: Vec<String>,
}

/// Passes over a function until its variables' widths settle: a variable assigned a vector from one read before
const MOST_WIDTH_PASSES: usize = 4;

impl Translation {
	/// `body` in WGSL f32 arithmetic of the parameter, number literals and the program's numbers: an f32 only
	fn expression(&mut self, body: &Node) -> Option<String> {
		let (code, width) = self.typed(body)?;
		if width != 1 {
			self.refuse(body, &format!(": a vector of {width} where WGSL needs a number (the dotted operators .+ .- .* ./ take vectors)"));
			return None;
		}
		Some(code)
	}

	/// `body` in WGSL with its width: numbers are f32, a list of 2 to 4 of them a vecNf (card compile-shaders); where
	/// WGSL cannot compute it, the refusal names it
	fn typed(&mut self, body: &Node) -> Option<Typed> {
		let typed = self.typed_as_written(body);
		if typed.is_none() {
			self.refuse(body, "");
		}
		typed
	}

	/// The refusal, unless an inner construct gave one before: `node` as written, its place, and why if known
	fn refuse(&mut self, node: &Node, why: &str) {
		if self.refusal.is_some() {
			return;
		}
		let written: String = node.serialize().split_whitespace().collect::<Vec<_>>().join(" ");
		let written = match written.chars().count() > REFUSAL_EXCERPT {
			true => format!("{}…", written.chars().take(REFUSAL_EXCERPT).collect::<String>()),
			false => written,
		};
		let place = crate::errors::diagnostic::position(node)
			.map(|(line, column)| format!(" at {line}:{column}"))
			.unwrap_or_default();
		self.refusal = Some(format!("`{written}`{place}{why}"));
	}

	fn typed_as_written(&mut self, body: &Node) -> Option<Typed> {
		if let Some((condition, then, otherwise)) = branches(body) {
			return self.selected(condition, then, otherwise);
		}
		if let Some((receiver, op, operand)) = crate::lowering::broadcasting::element_wise_parts(body) {
			return self.element_wise(&receiver, op, &operand);
		}
		let scalar = |code: String| Some((code, 1));
		match body.drop_meta() {
			Node::Symbol(name) if self.locals.contains(name) => {
				Some((format!("{VARIABLE_PREFIX}{name}"), self.widths.get(name).copied().unwrap_or(1)))
			}
			Node::Symbol(name) if *name == self.parameter => scalar(KERNEL_ITEM.to_string()),
			// `sun` of `sun() := …` (or `sun := …`): the call
			Node::Symbol(name) if program_function(name).is_some_and(|called| called.parameters.is_empty()) => {
				self.call_without_arguments(name)
			}
			Node::Symbol(name) => self.program_variable(name),
			// every number is an f32 on the GPU (user 2026-10-13: precision does not matter there)
			Node::Key(value, Op::As, _) => self.typed(value),
			Node::Number(number) if !matches!(number, crate::extensions::numbers::Number::Complex(..)) => {
				scalar(float_literal(f64::from(*number)))
			}
			Node::Key(empty, op, operand) if empty.is_empty_node() => self.prefixed(op, operand),
			Node::Key(base, Op::Pow, exponent) => scalar(self.power(base, exponent)?),
			Node::Key(vector, Op::Hash, index) => self.component(vector, index),
			Node::Key(vector, Op::Dot, field) => self.swizzle(vector, field.symbol_name()?),
			Node::Key(_, op, _) if comparison(*op).is_some() || matches!(op, Op::And | Op::Or) => {
				scalar(format!("select(0.0, 1.0, {})", self.condition(body)?))
			}
			Node::Key(left, op, right) => scalar(self.arithmetic(left, op, right)?),
			Node::List(items, crate::node::Bracket::Square, _) => self.vector(items),
			Node::List(items, _, _) => match items.as_slice() {
				// `center()` parses as `(center)`: a call when it names a program function of no parameters
				[single] => match single.symbol_name().and_then(|name| self.call_without_arguments(name)) {
					Some(called) => Some(called),
					None => self.typed(single),
				},
				[function, arguments @ ..] => self.function_call(&function.name(), arguments),
				_ => None,
			},
			_ => None,
		}
	}

	/// A variable of the program: a number, or in a fragment shader a vector of 2 to 4 numbers when the value the main
	/// level assigns it first is one (`camera = [0, 2.4, 0]`, `sun = normalize([…])`, card kernel-global-lists); the
	/// shader's values carry its value as a vecNf
	fn program_variable(&mut self, name: &str) -> Option<Typed> {
		let width = self.program_width(name);
		if width > 1 && !self.uniforms {
			self.refusal.get_or_insert(format!("`{name}`: a kernel reads the program's numbers, not its lists"));
			return None;
		}
		Some((self.program_number(name), width))
	}

	/// The width of the value the main level assigns the variable first: 1 where WGSL cannot compute it
	fn program_width(&mut self, name: &str) -> usize {
		if self.resolving.iter().any(|known| known == name) {
			return 1;
		}
		let Some(value) = program_value(name) else { return 1 };
		let mut resolving = self.resolving.clone();
		resolving.push(name.to_string());
		let mut translation = Translation { uniforms: self.uniforms, resolving, ..Translation::default() };
		translation.typed(&value).map_or(1, |(_, width)| width)
	}

	fn typed_all(&mut self, nodes: &[Node]) -> Option<Vec<Typed>> {
		nodes.iter().map(|node| self.typed(node)).collect()
	}

	/// A number of the program: a cell of the kernel's `data` after its items, or a fragment shader's value
	fn program_number(&mut self, name: &str) -> String {
		let index = self.outer.iter().position(|known| known == name).unwrap_or_else(|| {
			self.outer.push(name.to_string());
			self.outer.len() - 1
		});
		match self.uniforms {
			true => format!("{SHADER_VALUES}.{name}"),
			false => format!("data[{KERNEL_COUNT} + {index}u]"),
		}
	}

	fn prefixed(&mut self, op: &Op, operand: &Node) -> Option<Typed> {
		let (code, width) = self.typed(operand)?;
		let code = match op {
			Op::Neg => format!("(-{code})"),
			Op::Sqrt => format!("sqrt({code})"),
			Op::Abs => format!("abs({code})"),
			Op::Not => return Some((format!("select(0.0, 1.0, !{})", self.condition(operand)?), 1)),
			_ => return None,
		};
		Some((code, width))
	}

	/// A small whole power multiplied out, any other the heavy `pow`
	fn power(&mut self, base: &Node, exponent: &Node) -> Option<String> {
		match exponent.drop_meta() {
			Node::Number(crate::extensions::numbers::Number::Int(power))
				if (1..=LARGEST_MULTIPLIED_POWER).contains(power) =>
			{
				let base = self.expression(base)?;
				Some(format!("({})", vec![base; *power as usize].join(" * ")))
			}
			_ => {
				self.heavy = true;
				Some(format!("pow({}, {})", self.expression(base)?, self.expression(exponent)?))
			}
		}
	}

	/// Plain arithmetic of numbers: of lists it concatenates or repeats on the CPU, which no shader does
	fn arithmetic(&mut self, left: &Node, op: &Op, right: &Node) -> Option<String> {
		let symbol = operator_symbol(*op)?;
		Some(format!("({} {symbol} {})", self.expression(left)?, self.expression(right)?))
	}

	/// `a .op b`: WGSL's operator of vectors, of equal width, or of a vector and a number (decision S2)
	fn element_wise(&mut self, receiver: &Node, op: Op, operand: &Node) -> Option<Typed> {
		let [left, right] = [self.typed(receiver)?, self.typed(operand)?];
		let width = common_width(&[left.1, right.1])?;
		if op == Op::Pow {
			self.heavy = true;
			let [left, right] = [left, right].map(|side| splat(side, width));
			return Some((format!("pow({left}, {right})"), width));
		}
		Some((format!("({} {} {})", left.0, operator_symbol(op)?, right.0), width))
	}

	/// `p#1` … `p#4`: a vector's component; a computed index picks it at run time. An item of a program's list,
	/// `levels#band`, reads the shader's values, which hold one vec4f per item (whatever the list's length)
	fn component(&mut self, vector: &Node, index: &Node) -> Option<Typed> {
		let program_list = vector.symbol_name().filter(|list| {
			self.uniforms && !self.locals.iter().any(|local| local == list) && self.program_width(list) == 1
		});
		if let Some(list) = program_list {
			let (array, at) = (self.program_number(list), self.expression(index)?);
			if !self.item_lists.iter().any(|known| known == list) {
				self.item_lists.push(list.to_string());
			}
			return Some((format!("{array}[i32({at}) - 1].x"), 1));
		}
		let (vector, width) = self.typed(vector)?;
		if width == 1 {
			return None;
		}
		let code = match index.drop_meta() {
			Node::Number(crate::extensions::numbers::Number::Int(at)) => {
				let at = usize::try_from(*at).ok().filter(|at| (1..=width).contains(at))?;
				format!("{vector}.{}", COMPONENTS[at - 1])
			}
			_ => format!("{vector}[i32({}) - 1]", self.expression(index)?),
		};
		Some((code, 1))
	}

	/// `p.y`, `p.zyx`: WGSL's swizzle of the vector's components, as many wide as it names (decision S1)
	fn swizzle(&mut self, vector: &Node, name: &str) -> Option<Typed> {
		let components = crate::words::swizzle_components(name)?;
		let (vector, width) = self.typed(vector)?;
		let picks_within = width > 1 && components.iter().all(|component| *component < width);
		picks_within.then(|| (format!("{vector}.{name}"), components.len()))
	}

	/// `[a, b, c]` of numbers (and vectors): a vecNf of 2 to 4 components
	fn vector(&mut self, items: &[Node]) -> Option<Typed> {
		let items = self.typed_all(items)?;
		let width: usize = items.iter().map(|(_, width)| width).sum();
		let codes: Vec<String> = items.into_iter().map(|(code, _)| code).collect();
		(2..=COMPONENTS.len()).contains(&width).then(|| (format!("vec{width}f({})", codes.join(", ")), width))
	}

	/// A call of a WGSL math word, or of a program function
	fn function_call(&mut self, name: &str, arguments: &[Node]) -> Option<Typed> {
		let builtin = VECTOR_FUNCTIONS
			.iter()
			.find(|(word, ..)| *word == name)
			.map(|(_, wgsl, arity, result)| (*wgsl, *arity, *result))
			.or_else(|| KERNEL_FUNCTIONS.contains(&name).then_some((name, 1, 0)))
			.or_else(|| KERNEL_FUNCTIONS_OF_TWO.contains(&name).then_some((name, 2, 0)));
		let Some((wgsl, arity, result)) = builtin else {
			let called = program_function(name)?;
			return self.call(name, &called, arguments, false);
		};
		let arguments = self.typed_all(spread_arguments(arguments, arity)?)?;
		self.heavy |= HEAVY_FUNCTIONS.contains(&name) || wgsl == "pow";
		let width = common_width(&arguments.iter().map(|(_, width)| *width).collect::<Vec<_>>())?;
		let codes: Vec<String> = arguments.into_iter().map(|argument| splat(argument, width)).collect();
		Some((format!("{wgsl}({})", codes.join(", ")), if result == 0 { width } else { result }))
	}

	fn call_without_arguments(&mut self, name: &str) -> Option<Typed> {
		if self.locals.iter().any(|local| local == name) {
			return None;
		}
		let called = program_function(name).filter(|called| called.parameters.is_empty())?;
		self.call(name, &called, &[], false)
	}

	/// `condition ? then : otherwise` in an expression
	fn selected(&mut self, condition: &Node, then: &Node, otherwise: &Node) -> Option<Typed> {
		let condition = self.condition(condition)?;
		let [then, otherwise] = [self.typed(then)?, self.typed(otherwise)?];
		let width = common_width(&[then.1, otherwise.1])?;
		Some((format!("select({}, {}, {condition})", splat(otherwise, width), splat(then, width)), width))
	}

	/// A condition as a WGSL bool: comparisons, and, or, not; a number is true unless 0
	fn condition(&mut self, node: &Node) -> Option<String> {
		match node.drop_meta() {
			Node::Key(empty, Op::Not, operand) if empty.is_empty_node() => {
				Some(format!("!{}", self.condition(operand)?))
			}
			Node::Key(left, op @ (Op::And | Op::Or), right) => {
				let symbol = if *op == Op::And { "&&" } else { "||" };
				Some(format!("({} {symbol} {})", self.condition(left)?, self.condition(right)?))
			}
			Node::Key(left, op, right) if comparison(*op).is_some() => {
				Some(format!("({} {} {})", self.expression(left)?, comparison(*op)?, self.expression(right)?))
			}
			Node::List(items, _, _) if items.len() == 1 => self.condition(&items[0]),
			_ => Some(format!("({} != 0.0)", self.expression(node)?)),
		}
	}

	/// A call of the program's function `name`: translated once for its arguments' widths into a WGSL function; none
	/// for recursion
	fn call(&mut self, name: &str, called: &ProgramFunction, arguments: &[Node], color: bool) -> Option<Typed> {
		let arguments = spread_arguments(arguments, called.parameters.len())?;
		let arguments = self.typed_all(arguments)?;
		self.call_translated(name, called, arguments, color)
	}

	/// A call of the program's function `name` with its arguments in WGSL
	fn call_translated(
		&mut self,
		name: &str,
		called: &ProgramFunction,
		arguments: Vec<Typed>,
		color: bool,
	) -> Option<Typed> {
		if self.in_progress.iter().any(|caller| caller == name) {
			return None;
		}
		let widths: Vec<usize> = arguments.iter().map(|(_, width)| *width).collect();
		let vectors: String = match widths.iter().any(|width| *width > 1) {
			true => format!("_{}", widths.iter().map(usize::to_string).collect::<String>()),
			false => String::new(),
		};
		let translated_name = match color {
			true => format!("{FUNCTION_PREFIX}{name}{vectors}{COLOR_SUFFIX}"),
			false => format!("{FUNCTION_PREFIX}{name}{vectors}"),
		};
		if !self.translated.iter().any(|(translated, _)| *translated == translated_name) {
			self.in_progress.push(name.to_string());
			let function = self.function(&translated_name, called, &widths, color);
			self.in_progress.pop();
			let (function, width) = function?;
			self.functions.push(function);
			self.translated.push((translated_name.clone(), width));
		}
		let width = self.translated.iter().find(|(translated, _)| *translated == translated_name)?.1;
		let codes: Vec<String> = arguments.into_iter().map(|(code, _)| code).collect();
		Some((format!("{translated_name}({})", codes.join(", ")), width))
	}

	/// What a color-giving function returns: rgb(r, g, b) and the functions giving one as a vec4f; a packed
	/// 0xAARRGGBB literal unpacked exactly; a vector of 3 numbers from 0 to 1 that color; any other number ink, 0 or
	/// no paper (src/graphics/paint.rs pixel colors)
	fn color(&mut self, node: &Node) -> Option<String> {
		if let Some((condition, then, otherwise)) = branches(node) {
			return self.selected_color(condition, then, otherwise);
		}
		match node.drop_meta() {
			Node::Number(number) if f64::from(*number) >= crate::graphics::pixels::COLOR_FROM as f64 => {
				let packed = f64::from(*number) as u64;
				let channel = |shift: u32| float_literal(((packed >> shift) & 0xFF) as f64 / 255.0);
				Some(format!("vec4f({}, {}, {}, 1.0)", channel(16), channel(8), channel(0)))
			}
			Node::List(items, _, _) if items.len() == 1 => self.color(&items[0]),
			Node::List(items, _, _) if !items.is_empty() && items[0].is_symbol(RGB_WORD) => {
				let channels = self.expressions(spread_arguments(&items[1..], 3)?)?;
				Some(format!("(vec4f({}, 255.0) / 255.0)", channels.join(", ")))
			}
			Node::List(items, _, _)
				if !items.is_empty() && program_function(&items[0].name()).is_some_and(|f| gives_color(&f.body)) =>
			{
				let called = program_function(&items[0].name())?;
				self.call(&items[0].name(), &called, &items[1..], true).map(|(code, _)| code)
			}
			_ => match self.typed(node)? {
				(code, 1) => {
					let (ink, paper) = (gray(crate::graphics::pixels::INK), gray(crate::graphics::pixels::PAPER));
					Some(format!("select({ink}, {paper}, {code} == 0.0)"))
				}
				(code, 3) => Some(format!("vec4f({code}, 1.0)")),
				// red, green, blue and alpha
				(code, 4) => Some(code),
				(_, width) => {
					self.refuse(node, &format!(": a color is a number or 3 or 4 channels, not {width}"));
					None
				}
			},
		}
	}

	fn expressions(&mut self, nodes: &[Node]) -> Option<Vec<String>> {
		nodes.iter().map(|node| self.expression(node)).collect()
	}

	fn selected_color(&mut self, condition: &Node, then: &Node, otherwise: &Node) -> Option<String> {
		let condition = self.condition(condition)?;
		Some(format!("select({}, {}, {condition})", self.color(otherwise)?, self.color(then)?))
	}

	/// A function's value as its WGSL function returns it, a color or a number or vector: `return …;`, the width
	/// noted (every return of a function gives one width)
	fn returning(&mut self, node: &Node) -> Option<String> {
		let (code, width) = match self.color_result {
			true => (self.color(node)?, COMPONENTS.len()),
			false => self.typed(node)?,
		};
		if *self.returned_width.get_or_insert(width) != width {
			self.refuse(node, ": the function's returns give values of different widths");
			return None;
		}
		Some(format!("return {code};\n"))
	}

	/// The WGSL function of the program's function `name` for arguments of those widths, and its result's width: its
	/// parameters copied into variables, every variable it assigns declared first (of the width its values have), its
	/// statements, the last one's value returned
	fn function(
		&mut self,
		translated_name: &str,
		called: &ProgramFunction,
		widths: &[usize],
		color: bool,
	) -> Option<(String, usize)> {
		let mut variables = called.parameters.clone();
		assigned_names(&called.body, &mut variables);
		let parameter_widths: HashMap<String, usize> =
			called.parameters.iter().cloned().zip(widths.iter().copied()).collect();
		let caller = (
			std::mem::take(&mut self.parameter),
			std::mem::replace(&mut self.locals, variables),
			std::mem::replace(&mut self.widths, parameter_widths.clone()),
			std::mem::replace(&mut self.color_result, color),
			self.returned_width.take(),
		);
		// a variable's width is known once a pass assigned it: passes until the widths settle
		let mut body = None;
		for _ in 0..MOST_WIDTH_PASSES {
			let before = self.widths.clone();
			self.returned_width = None;
			let mut out = String::new();
			body = self.returned(&called.body, &mut out).map(|_| out);
			if body.is_none() || self.widths == before {
				break;
			}
			body = None;
		}
		let (widths, returned_width) = (std::mem::take(&mut self.widths), self.returned_width);
		let declared: String = self
			.locals
			.iter()
			.map(|variable| {
				let wgsl = wgsl_type(widths.get(variable).copied().unwrap_or(1));
				match called.parameters.contains(variable) {
					true => format!("\tvar {VARIABLE_PREFIX}{variable} = {PARAMETER_PREFIX}{variable};\n"),
					false => format!("\tvar {VARIABLE_PREFIX}{variable}: {wgsl} = {wgsl}();\n"),
				}
			})
			.collect();
		(self.parameter, self.locals, self.widths, self.color_result, self.returned_width) = caller;
		let (body, returned_width) = (body?, returned_width?);
		let parameters: Vec<String> = called
			.parameters
			.iter()
			.zip(widths_of(&called.parameters, &parameter_widths))
			.map(|(parameter, width)| format!("{PARAMETER_PREFIX}{parameter}: {}", wgsl_type(width)))
			.collect();
		let function = format!(
			"fn {translated_name}({}) -> {} {{\n{declared}{}}}",
			parameters.join(", "),
			wgsl_type(returned_width),
			indented(&body)
		);
		Some((function, returned_width))
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
			Node::Key(target, op, _) if is_binding(op) && matches!(target.drop_meta(), Node::Symbol(_)) => {
				self.statement(node, out)?;
				*out += &self.returning(target)?;
				Some(())
			}
			_ if returned_value(node).is_some() => self.statement(node, out),
			_ if is_statement_only(node) => {
				self.refuse(node, ": a function's last statement gives no value");
				None
			}
			_ => {
				*out += &self.returning(node)?;
				Some(())
			}
		}
	}

	/// One statement of a function in WGSL into `out`: an assignment, if, while, `for i in a..b`, break, return, a block
	fn statement(&mut self, node: &Node, out: &mut String) -> Option<()> {
		if let Some(statements) = block_statements(node) {
			return statements.iter().try_for_each(|statement| self.statement(statement, out));
		}
		if let Some(value) = returned_value(node) {
			*out += &self.returning(value)?;
			return Some(());
		}
		if node.is_symbol(BREAK_WORD) {
			*out += "break;\n";
			return Some(());
		}
		if let Some((variable, from, op, to, body)) = range_loop(node) {
			self.heavy = true;
			let (variable, from, to) =
				(format!("{VARIABLE_PREFIX}{variable}"), self.expression(from)?, self.expression(to)?);
			let below = if op == Op::To { "<=" } else { "<" };
			*out += &format!("for ({variable} = {from}; {variable} {below} {to}; {variable} = {variable} + 1.0) {{\n");
			self.statement(body, out)?;
			*out += "}\n";
			return Some(());
		}
		match node.drop_meta() {
			Node::Key(target, op, value) if is_binding(op) => {
				let name = target.symbol_name()?;
				let (value, width) = match op {
					Op::Assign | Op::Define => self.typed(value)?,
					compound => self.typed(&Node::Key(target.clone(), compound.base_op(), value.clone()))?,
				};
				self.widths.insert(name.to_string(), width);
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
				if !empty.is_empty_node() {
					return None;
				}
				self.heavy = true;
				*out += &format!("while {} {{\n", self.condition(condition)?);
				self.statement(body, out)?;
				*out += "}\n";
				Some(())
			}
			// a value nothing uses: checked, then dropped
			_ => self.typed(node).map(|_| ()),
		}
	}
}

/// A WGSL value and its width: 1 an f32, 2 to 4 a vecNf
type Typed = (String, usize);

/// The width two or more values combine to: equal widths, or numbers beside vectors of one width
fn common_width(widths: &[usize]) -> Option<usize> {
	let widest = widths.iter().copied().max()?;
	widths.iter().all(|width| *width == 1 || *width == widest).then_some(widest)
}

/// A value as `width` wide: a number repeated into a vector
fn splat((code, from): Typed, width: usize) -> String {
	match from == width {
		true => code,
		false => format!("{}({code})", wgsl_type(width)),
	}
}

fn wgsl_type(width: usize) -> String {
	match width {
		1 => "f32".to_string(),
		vector => format!("vec{vector}f"),
	}
}

fn widths_of<'a>(names: &'a [String], widths: &'a HashMap<String, usize>) -> impl Iterator<Item = usize> + 'a {
	names.iter().map(|name| widths.get(name).copied().unwrap_or(1))
}

/// WGSL's arithmetic operator of a warp one
fn operator_symbol(op: Op) -> Option<&'static str> {
	Some(match op {
		Op::Add => "+",
		Op::Sub => "-",
		Op::Mul => "*",
		Op::Div => "/",
		Op::Mod => "%",
		_ => return None,
	})
}

/// A call's `count` arguments: `f(a, b)` gives them as one list
fn spread_arguments(arguments: &[Node], count: usize) -> Option<&[Node]> {
	let arguments = match arguments {
		[together] if count > 1 => match together.drop_meta() {
			Node::List(items, _, _) => items.as_slice(),
			_ => return None,
		},
		separate => separate,
	};
	(arguments.len() == count).then_some(arguments)
}

/// A function body's statements, one tab in and one more inside each `{ … }` (shader_of shows them)
fn indented(body: &str) -> String {
	let mut depth = 1;
	let mut out = String::new();
	for line in body.lines() {
		depth -= line.starts_with('}') as usize;
		out += &format!("{}{line}\n", "\t".repeat(depth));
		depth += line.ends_with('{') as usize;
	}
	out
}

/// A gray of the paper or ink shade as a WGSL color
fn gray(shade: u8) -> String {
	let level = float_literal(shade as f64 / 255.0);
	format!("vec4f({level}, {level}, {level}, 1.0)")
}

/// Whether a function's body gives a color: a value it returns is rgb(…), a packed color literal or a call of a
/// function giving one
fn gives_color(body: &Node) -> bool {
	gives_color_within(body, &mut vec![])
}

fn gives_color_within(body: &Node, seen: &mut Vec<String>) -> bool {
	let mut values = vec![];
	returned_values(body, &mut values);
	values.iter().any(|value| match value.drop_meta() {
		Node::Number(number) => f64::from(*number) >= crate::graphics::pixels::COLOR_FROM as f64,
		Node::List(items, _, _) if !items.is_empty() => {
			let name = items[0].name();
			name == RGB_WORD
				|| (!seen.contains(&name) && {
					seen.push(name.clone());
					program_function(&name).is_some_and(|called| gives_color_within(&called.body, seen))
				})
		}
		_ => false,
	})
}

/// The values a body gives: its last statement's (each branch's), and every `return`'s
fn returned_values<'a>(body: &'a Node, values: &mut Vec<&'a Node>) {
	body.visit(&mut |part| {
		if let Some(value) = returned_value(part) {
			values.push(value);
		}
	});
	last_values(body, values);
}

fn last_values<'a>(node: &'a Node, values: &mut Vec<&'a Node>) {
	if let Some(statements) = block_statements(node) {
		return statements.last().into_iter().for_each(|last| last_values(last, values));
	}
	if let Some((_, then, otherwise)) = branches(node) {
		last_values(then, values);
		return last_values(otherwise, values);
	}
	match node.drop_meta() {
		Node::List(items, _, _) if items.len() == 1 => last_values(&items[0], values),
		_ => values.push(node),
	}
}

/// The prelude word painted_functions may call where the program writes none (modules IMPLIED_WORDS): pixel_color,
/// of `paint(f, …)`
pub(crate) fn implied_word(node: &Node) -> Option<&'static str> {
	let paints = match node.drop_meta() {
		Node::List(items, _, _) => match items.as_slice() {
			[paint, arguments @ ..] if paint.is_symbol(PAINT_WORD) => {
				spread_arguments(arguments, 3).is_some_and(|arguments| arguments[0].symbol_name().is_some())
			}
			_ => false,
		},
		_ => false,
	};
	paints.then_some(PIXEL_COLOR)
}

/// Every `paint(f, width, height)` of a warp function f(x, y): a fragment shader of f where WGSL can compute it, the
/// program's numbers it reads as the shader's values; otherwise f of every pixel on the CPU, row by row
pub(crate) fn painted_functions(node: Node) -> Node {
	if let Some(painted) = painted_function(&node) {
		return painted;
	}
	node.map_children(painted_functions)
}

fn painted_function(node: &Node) -> Option<Node> {
	let Node::List(items, _, _) = node.drop_meta() else {
		return None;
	};
	let [paint, arguments @ ..] = items.as_slice() else {
		return None;
	};
	let on_cpu =
		switched_off() || crate::parallel::is_annotated(node, CPU_ATTRIBUTE) || node.attribute(CPU_ATTRIBUTE).is_some();
	if paint.is_symbol(SHADER_OF_WORD) {
		return shader_of(arguments);
	}
	if !paint.is_symbol(PAINT_WORD) {
		return None;
	}
	let [function, width, height] = spread_arguments(arguments, 3)? else {
		return None;
	};
	let name = function.drop_meta().name();
	let painted = program_function(&name).filter(|called| called.parameters.len() == 2)?;
	let (width, height) = (width.serialize(), height.serialize());
	let shader = match on_cpu {
		true => Err(String::new()),
		false => fragment_shader(&name, &painted),
	};
	if let Some(reason) = shader.as_ref().err().filter(|reason| !reason.is_empty()) {
		let refused = crate::errors::diagnostic::Diagnostic::at(
			function,
			format!("paint: WGSL cannot compute {name} ({reason}), so the CPU paints it pixel by pixel"),
		)
		.weak();
		if let Err(error) = crate::errors::diagnostic::report(&[refused]) {
			return Some(error);
		}
	}
	let lowered = match shader {
		Ok((shader, entries)) => {
			let values = match entries.is_empty() {
				true => String::new(),
				false => format!(", {{{}}}", entries.join(", ")),
			};
			let on_gpu = format!("{PAINT_WORD}({}, {width}, {height}{values})", Node::Text(shader).serialize());
			// card paint-function: without a GPU to paint the shader, f of every pixel on the CPU, as a map falls back
			format!(
				"if system_value({}) != 0 {{ {on_gpu} }} else {{ {} }}",
				Node::Text(GPU_PAINTS.into()).serialize(),
				on_cpu_painted(&name, &width, &height)
			)
		}
		Err(_) => on_cpu_painted(&name, &width, &height),
	};
	let lowered = expanded_paint_each(parsed_statement(&lowered));
	// the prelude's name of pixel_color, which no parse gives (library_words names prelude words after this pass)
	let pixel_color = crate::modules::prelude_name(PIXEL_COLOR).map(Node::Symbol)?;
	Some(crate::law::substitute(&lowered, &HashMap::from([(PIXEL_COLOR.to_string(), pixel_color)])))
}

/// One generated statement, not the list of one statement a parse gives
fn parsed_statement(code: &str) -> Node {
	match crate::warp_parser::parse_generated(code).drop_meta() {
		Node::List(statements, crate::node::Bracket::None, _) if statements.len() == 1 => statements[0].clone(),
		single => single.clone(),
	}
}

/// paint of f(x, y) at every pixel, computed on the CPU row by row: the prelude's paint_each, which expanded_paint_each
/// writes in place
fn on_cpu_painted(name: &str, width: &str, height: &str) -> String {
	format!("{PAINT_EACH}({name}, ({width}), ({height}))")
}

/// `paint_each(f, width, height)` as the body of the prelude's paint_each with its parameters bound to the call's
/// arguments, so f is called by its name: called as a value its parameters lose their types (twice as slow). Card
/// specialise-named-functions makes this the general rule for higher-order words
fn expanded_paint_each(node: Node) -> Node {
	let call = match node.drop_meta() {
		Node::List(items, _, _) => match items.as_slice() {
			[word, arguments @ ..] if word.is_symbol(PAINT_EACH) => Some(arguments.to_vec()),
			_ => None,
		},
		_ => None,
	};
	let Some(arguments) = call else {
		return node.map_children(expanded_paint_each);
	};
	let (parameters, body) =
		crate::modules::prelude_definition(PAINT_EACH).expect("lib/prelude.warp defines paint_each");
	let arguments =
		spread_arguments(&arguments, parameters.len()).expect("on_cpu_painted passes paint_each's arguments");
	let bindings = parameters.iter().map(|parameter| parameter.drop_meta().name()).zip(arguments.iter().cloned());
	crate::lowering::definitions::inlining::substituted_hygienically(&body, &bindings.collect())
}

/// `shader_of(f)`: the WGSL fragment shader paint(f, width, height) compiles f(x, y) to, as a text; an error where WGSL
/// cannot compute f (then paint computes it on the CPU)
fn shader_of(arguments: &[Node]) -> Option<Node> {
	let [function] = spread_arguments(arguments, 1)? else {
		return None;
	};
	let name = function.drop_meta().symbol_name()?.to_string();
	let painted = program_function(&name).filter(|called| called.parameters.len() == 2)?;
	Some(match fragment_shader(&name, &painted) {
		Ok((shader, _)) => Node::Text(shader),
		Err(reason) => crate::warp_parser::parse_generated(&format!(
			"raise {}",
			Node::Text(format!(
				"{SHADER_OF_WORD}: WGSL cannot compute {name} ({reason}), paint computes it on the CPU"
			))
			.serialize()
		)),
	})
}

/// The fragment shader painting each pixel the color of `function` at its column and row, and the entries of its values:
/// the program's numbers it reads, a list it reads item by item as a list of one-number lists (one vec4f per item);
/// where WGSL cannot compute the function, the construct it cannot (card kernel-refusal-reason)
fn fragment_shader(name: &str, function: &ProgramFunction) -> Result<(String, Vec<String>), String> {
	let mut translation = Translation { uniforms: true, ..Translation::default() };
	let pixel = vec![("floor(at.x)".to_string(), 1), ("floor(at.y)".to_string(), 1)];
	let Some((painted, _)) = translation.call_translated(name, function, pixel, true) else {
		return Err(translation.refusal.unwrap_or_else(|| "a construct outside the shader subset".to_string()));
	};
	let functions: String = translation.functions.iter().map(|function| format!("{function}\n")).collect();
	let shader = format!(
		"{functions}@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f {{\n\treturn {painted};\n}}"
	);
	let entries = translation.outer.iter().map(|name| match translation.item_lists.contains(name) {
		true => format!("{name}: [[item] for item in {name}]"),
		false => format!("{name}: {name}"),
	});
	Ok((shader, entries.collect()))
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
		Node::List(items, _, _) if is_return(items) => Some(&items[1]),
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
	let variable = variable.symbol_name()?;
	match range.drop_meta() {
		Node::Key(from, op @ (Op::Range | Op::To), to) => Some((variable.to_string(), from, *op, to, body)),
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
		Node::Key(target, op, _) if is_binding(op) => {
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

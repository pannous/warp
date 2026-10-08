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
/// The lambda's parameter as the kernel names it (the user's name could be a WGSL keyword)
const KERNEL_ITEM: &str = "item";
/// The math words WGSL has under the same name and meaning (warp's log is the natural logarithm, as WGSL's)
const KERNEL_FUNCTIONS: [&str; 15] = ["sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "exp", "log", "sqrt", "abs", "floor", "ceil"];
const KERNEL_FUNCTIONS_OF_TWO: [&str; 3] = ["min", "max", "atan2"];
/// `x^k` for a whole k up to this is a product (pow of a negative base is NaN in WGSL, a number on the CPU)
const LARGEST_MULTIPLIED_POWER: i64 = 8;

/// `xs.map(f) @gpu` or `@gpu xs.map(f)`: xs and f
pub(crate) fn gpu_map(value: &Node) -> Option<(Node, Node)> {
	crate::parallel::is_annotated(value, GPU_ATTRIBUTE).then(|| crate::parallel::map_call(value)).flatten()
}

/// The WGSL compute shader mapping every number of `data` by the lambda `parameter => body`, if WGSL can compute it
pub(crate) fn kernel(lambda: &Node) -> Option<String> {
	let Node::Key(parameter, Op::FatArrow, body) = lambda.drop_meta() else { return None };
	let Node::Symbol(parameter) = parameter.drop_meta() else { return None };
	let computed = expression(parameter, body)?;
	Some(format!("@group(0) @binding(0) var<storage, read_write> data: array<f32>;
@compute @workgroup_size({WORKGROUP_SIZE}) fn main(@builtin(global_invocation_id) id: vec3<u32>) {{
	if (id.x < arrayLength(&data)) {{ let {KERNEL_ITEM} = data[id.x]; data[id.x] = {computed}; }}
}}"))
}

/// The kernel of a lambda worth the GPU: one the CPU's f64x2 kernel computes is faster there at any count
pub(crate) fn gpu_kernel(lambda: &Node) -> Option<String> {
	kernel(lambda).filter(|_| !is_light(lambda))
}

/// Arithmetic the CPU maps two cells at a time (wasm_emitter/linear_arrays.rs)
fn is_light(lambda: &Node) -> bool {
	matches!(lambda.drop_meta(), Node::Key(parameter, Op::FatArrow, body)
		if crate::wasm_emitter::linear_arrays::is_float_kernel(&parameter.name(), body))
}

/// `body` in WGSL f32 arithmetic of the parameter and number literals
fn expression(parameter: &str, body: &Node) -> Option<String> {
	let of = |node: &Node| expression(parameter, node);
	match body.drop_meta() {
		Node::Symbol(name) if name == parameter => Some(KERNEL_ITEM.to_string()),
		Node::Number(number) if !matches!(number, crate::extensions::numbers::Number::Complex(..)) => Some(float_literal(f64::from(*number))),
		Node::Key(empty, op, operand) if matches!(empty.drop_meta(), Node::Empty) => match op {
			Op::Neg => Some(format!("(-{})", of(operand)?)),
			Op::Sqrt => Some(format!("sqrt({})", of(operand)?)),
			Op::Abs => Some(format!("abs({})", of(operand)?)),
			_ => None,
		},
		Node::Key(base, Op::Pow, exponent) => match exponent.drop_meta() {
			Node::Number(crate::extensions::numbers::Number::Int(power)) if (1..=LARGEST_MULTIPLIED_POWER).contains(power) => {
				let base = of(base)?;
				Some(format!("({})", vec![base; *power as usize].join(" * ")))
			}
			_ => Some(format!("pow({}, {})", of(base)?, of(exponent)?)),
		},
		Node::Key(left, op, right) => {
			let symbol = match op {
				Op::Add => "+",
				Op::Sub => "-",
				Op::Mul => "*",
				Op::Div => "/",
				_ => return None,
			};
			Some(format!("({} {symbol} {})", of(left)?, of(right)?))
		}
		Node::List(items, _, _) => match items.as_slice() {
			[single] => of(single),
			[function, argument] if KERNEL_FUNCTIONS.contains(&function.name().as_str()) => Some(format!("{}({})", function.name(), of(argument)?)),
			[function, first, second] if KERNEL_FUNCTIONS_OF_TWO.contains(&function.name().as_str()) => Some(format!("{}({}, {})", function.name(), of(first)?, of(second)?)),
			// `max(x, 1)`: the arguments as one list
			[function, arguments] if KERNEL_FUNCTIONS_OF_TWO.contains(&function.name().as_str()) => match arguments.drop_meta() {
				Node::List(pair, _, _) if let [first, second] = pair.as_slice() => Some(format!("{}({}, {})", function.name(), of(first)?, of(second)?)),
				_ => None,
			},
			_ => None,
		},
		_ => None,
	}
}

/// A WGSL f32 literal: always with a point or an exponent, so `2` is not an i32
fn float_literal(number: f64) -> String {
	let written = format!("{:?}", number as f32);
	if written.contains(['.', 'e', 'E']) { written } else { format!("{written}.0") }
}

/// Every `@gpu` that shared_arrays will not lower to the GPU (`linear_floats` are the program's linear float arrays): a
/// warning saying why it runs on the CPU (an error under strict)
pub fn warn_unapplied(node: Node, linear_floats: &[String]) -> Node {
	let mut warnings = vec![];
	collect_unapplied(&node, linear_floats, &mut warnings);
	match crate::diagnostic::report(&warnings) {
		Ok(()) => node,
		Err(error) => error,
	}
}

fn collect_unapplied(node: &Node, linear_floats: &[String], warnings: &mut Vec<crate::diagnostic::Diagnostic>) {
	let warn = |warnings: &mut Vec<_>, reason: String| warnings.push(crate::diagnostic::Diagnostic::at(node, format!("{reason}, so this runs on the CPU")));
	match node.drop_meta() {
		Node::Key(target, Op::Assign, value) if matches!(target.drop_meta(), Node::Symbol(_)) && gpu_map(value).is_some() => {
			if let Some(reason) = unapplied(value, linear_floats) {
				warn(warnings, reason);
			}
		}
		_ if crate::parallel::annotated_here(node, GPU_ATTRIBUTE) => {
			let assign = if crate::parallel::map_call(node).is_some() { "@gpu maps into a new array: assign the map" } else { "@gpu runs only a map" };
			warn(warnings, format!("{assign}, `ys = xs.map(x => …) @gpu`"));
		}
		Node::Key(left, _, right) => [left, right].into_iter().for_each(|part| collect_unapplied(part, linear_floats, warnings)),
		Node::List(items, _, _) => items.iter().for_each(|item| collect_unapplied(item, linear_floats, warnings)),
		_ => {}
	}
}

/// Why the `@gpu` map `value` cannot run on the GPU, if it cannot
fn unapplied(value: &Node, linear_floats: &[String]) -> Option<String> {
	let (array, lambda) = gpu_map(value)?;
	if !matches!(array.drop_meta(), Node::Symbol(name) if linear_floats.contains(name)) {
		return Some(format!("@gpu maps a `linear xs = float[n]` on the GPU; {} is none", array.serialize().trim()));
	}
	if is_light(&lambda) {
		return Some(format!("@gpu: the CPU's f64x2 kernel maps `{}` faster (~1 ns an item, the GPU ~12)", lambda.serialize().trim()));
	}
	kernel(&lambda).is_none().then(|| format!("@gpu: WGSL cannot compute `{}` (only + - * / ^ √ and math words of the item and numbers)", lambda.serialize().trim()))
}

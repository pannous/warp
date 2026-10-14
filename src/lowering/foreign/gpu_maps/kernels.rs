//! WGSL from warp: a numeric lambda as a compute kernel (with its reductions), a painted function as a fragment shader
use super::*;

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
	let Node::Symbol(parameter) = parameter.drop_meta() else {
		return None;
	};
	let mut translation = Translation { parameter: parameter.clone(), ..Translation::default() };
	let computed = translation.expression(body)?;
	let Translation { outer, heavy, functions, .. } = translation;
	Some(Kernel {
		computed,
		outer,
		heavy,
		functions: functions.into_iter().map(|function| format!("\n{function}")).collect(),
	})
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
	let functions = statements
		.iter()
		.filter_map(crate::law::function_definition)
		.map(|definition| {
			let parameters = definition.parameters.into_iter().map(|(name, _)| name).collect();
			(definition.name, ProgramFunction { parameters, body: definition.body })
		})
		.collect();
	crate::compiler::compile_context::scoped(|context| &mut context.program_functions, functions, run)
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
	translated: Vec<String>,
	/// The functions being translated: a call of one of them is recursion, which WGSL forbids
	in_progress: Vec<String>,
	/// The program's numbers are a fragment shader's `values` (paint), not cells of the kernel's `data`
	uniforms: bool,
	/// The function being translated gives a color, a vec4f
	color_result: bool,
}

impl Translation {
	/// `body` in WGSL f32 arithmetic of the parameter, number literals and the program's numbers
	fn expression(&mut self, body: &Node) -> Option<String> {
		if let Some((condition, then, otherwise)) = branches(body) {
			return self.selected(condition, then, otherwise);
		}
		match body.drop_meta() {
			Node::Symbol(name) if self.locals.contains(name) => Some(format!("{VARIABLE_PREFIX}{name}")),
			Node::Symbol(name) if *name == self.parameter => Some(KERNEL_ITEM.to_string()),
			Node::Symbol(name) => Some(self.program_number(name)),
			// every number is an f32 on the GPU (user 2026-10-13: precision does not matter there)
			Node::Key(value, Op::As, _) => self.expression(value),
			Node::Number(number) if !matches!(number, crate::extensions::numbers::Number::Complex(..)) => {
				Some(float_literal(f64::from(*number)))
			}
			Node::Key(empty, op, operand) if empty.is_empty_node() => self.prefixed(op, operand),
			Node::Key(base, Op::Pow, exponent) => self.power(base, exponent),
			Node::Key(_, op, _) if comparison(*op).is_some() || matches!(op, Op::And | Op::Or) => {
				Some(format!("select(0.0, 1.0, {})", self.condition(body)?))
			}
			Node::Key(left, op, right) => self.arithmetic(left, op, right),
			Node::List(items, _, _) => match items.as_slice() {
				[single] => self.expression(single),
				[function, arguments @ ..] => self.function_call(&function.name(), arguments),
				_ => None,
			},
			_ => None,
		}
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

	fn prefixed(&mut self, op: &Op, operand: &Node) -> Option<String> {
		match op {
			Op::Neg => Some(format!("(-{})", self.expression(operand)?)),
			Op::Sqrt => Some(format!("sqrt({})", self.expression(operand)?)),
			Op::Abs => Some(format!("abs({})", self.expression(operand)?)),
			Op::Not => Some(format!("select(0.0, 1.0, !{})", self.condition(operand)?)),
			_ => None,
		}
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

	fn arithmetic(&mut self, left: &Node, op: &Op, right: &Node) -> Option<String> {
		let symbol = match op {
			Op::Add => "+",
			Op::Sub => "-",
			Op::Mul => "*",
			Op::Div => "/",
			Op::Mod => "%",
			_ => return None,
		};
		Some(format!("({} {symbol} {})", self.expression(left)?, self.expression(right)?))
	}

	/// A call of a program function, or of a WGSL function of one or two arguments
	fn function_call(&mut self, name: &str, arguments: &[Node]) -> Option<String> {
		if let Some(called) = program_function(name) {
			return self.call(name, &called, arguments, false);
		}
		let arity = if KERNEL_FUNCTIONS.contains(&name) {
			1
		} else if KERNEL_FUNCTIONS_OF_TWO.contains(&name) {
			2
		} else {
			return None;
		};
		let arguments = spread_arguments(arguments, arity)?;
		self.heavy |= HEAVY_FUNCTIONS.contains(&name);
		let arguments = self.expressions(arguments)?;
		Some(format!("{name}({})", arguments.join(", ")))
	}

	fn expressions(&mut self, nodes: &[Node]) -> Option<Vec<String>> {
		nodes.iter().map(|node| self.expression(node)).collect()
	}

	/// `condition ? then : otherwise` in an expression
	fn selected(&mut self, condition: &Node, then: &Node, otherwise: &Node) -> Option<String> {
		let condition = self.condition(condition)?;
		Some(format!("select({}, {}, {condition})", self.expression(otherwise)?, self.expression(then)?))
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

	/// A call of the program's function `name`: translated once into a WGSL function; none for recursion
	fn call(&mut self, name: &str, called: &ProgramFunction, arguments: &[Node], color: bool) -> Option<String> {
		let arguments = spread_arguments(arguments, called.parameters.len())?;
		let arguments = self.expressions(arguments)?;
		self.call_translated(name, called, arguments, color)
	}

	/// A call of the program's function `name` with its arguments in WGSL
	fn call_translated(
		&mut self,
		name: &str,
		called: &ProgramFunction,
		arguments: Vec<String>,
		color: bool,
	) -> Option<String> {
		if self.in_progress.iter().any(|caller| caller == name) {
			return None;
		}
		let translated_name = match color {
			true => format!("{FUNCTION_PREFIX}{name}{COLOR_SUFFIX}"),
			false => format!("{FUNCTION_PREFIX}{name}"),
		};
		if !self.translated.contains(&translated_name) {
			self.in_progress.push(name.to_string());
			let function = self.function(&translated_name, called, color);
			self.in_progress.pop();
			self.functions.push(function?);
			self.translated.push(translated_name.clone());
		}
		Some(format!("{translated_name}({})", arguments.join(", ")))
	}

	/// What a color-giving function returns: rgb(r, g, b) and the functions giving one as a vec4f; a packed
	/// 0xAARRGGBB literal unpacked exactly; any other number ink, 0 or no paper (src/graphics/paint.rs pixel colors)
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
				self.call(&items[0].name(), &called, &items[1..], true)
			}
			_ => {
				let (ink, paper) = (gray(crate::graphics::pixels::INK), gray(crate::graphics::pixels::PAPER));
				Some(format!("select({ink}, {paper}, {} == 0.0)", self.expression(node)?))
			}
		}
	}

	fn selected_color(&mut self, condition: &Node, then: &Node, otherwise: &Node) -> Option<String> {
		let condition = self.condition(condition)?;
		Some(format!("select({}, {}, {condition})", self.color(otherwise)?, self.color(then)?))
	}

	/// A function's value as its WGSL function returns it: a color or a number
	fn result(&mut self, node: &Node) -> Option<String> {
		match self.color_result {
			true => self.color(node),
			false => self.expression(node),
		}
	}

	/// The WGSL function of the program's function `name`: its parameters copied into variables, every variable it
	/// assigns declared first, its statements, the last one's value returned
	fn function(&mut self, translated_name: &str, called: &ProgramFunction, color: bool) -> Option<String> {
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
			std::mem::replace(&mut self.color_result, color),
		);
		let mut body = String::new();
		let returned = self.returned(&called.body, &mut body);
		(self.parameter, self.locals, self.color_result) = caller;
		returned?;
		let parameters: Vec<String> =
			called.parameters.iter().map(|parameter| format!("{PARAMETER_PREFIX}{parameter}: f32")).collect();
		Some(format!(
			"fn {translated_name}({}) -> {} {{\n{declared}{body}}}",
			parameters.join(", "),
			if color { "vec4f" } else { "f32" }
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
			Node::Key(target, op, _) if is_assigning_op(*op) && matches!(target.drop_meta(), Node::Symbol(_)) => {
				self.statement(node, out)?;
				*out += &format!("return {};\n", self.result(target)?);
				Some(())
			}
			_ if returned_value(node).is_some() => self.statement(node, out),
			_ if is_statement_only(node) => None,
			_ => {
				*out += &format!("return {};\n", self.result(node)?);
				Some(())
			}
		}
	}

	/// One statement of a function in WGSL into `out`: an assignment, if, while, `for i in a..b`, return, a block
	fn statement(&mut self, node: &Node, out: &mut String) -> Option<()> {
		if let Some(statements) = block_statements(node) {
			return statements.iter().try_for_each(|statement| self.statement(statement, out));
		}
		if let Some(value) = returned_value(node) {
			*out += &format!("return {};\n", self.result(value)?);
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
			Node::Key(target, op, value) if is_assigning_op(*op) => {
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
			_ => self.expression(node).map(|_| ()),
		}
	}
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
	if !paint.is_symbol(PAINT_WORD) {
		return None;
	}
	let [function, width, height] = spread_arguments(arguments, 3)? else {
		return None;
	};
	let name = function.drop_meta().name();
	let painted = program_function(&name).filter(|called| called.parameters.len() == 2)?;
	let (width, height) = (width.serialize(), height.serialize());
	let lowered = match fragment_shader(&name, &painted).filter(|_| !on_cpu) {
		Some((shader, outer)) => {
			let entries: Vec<String> = outer.iter().map(|name| format!("{name}: {name}")).collect();
			let values = match entries.is_empty() {
				true => String::new(),
				false => format!(", {{{}}}", entries.join(", ")),
			};
			format!("{PAINT_WORD}({}, {width}, {height}{values})", Node::Text(shader).serialize())
		}
		None => format!(
			"{PAINT_WORD}([{name}(i % ({width}), i//({width})) for i in 0..(({width}) * ({height}))], {width}, {height})"
		),
	};
	match crate::warp_parser::parse(&lowered).drop_meta() {
		Node::List(statements, crate::node::Bracket::None, _) if statements.len() == 1 => Some(statements[0].clone()),
		single => Some(single.clone()),
	}
}

/// The fragment shader painting each pixel the color of `function` at its column and row, and the program's numbers
/// it reads; none where WGSL cannot compute the function
fn fragment_shader(name: &str, function: &ProgramFunction) -> Option<(String, Vec<String>)> {
	let mut translation = Translation { uniforms: true, ..Translation::default() };
	let pixel = vec!["floor(at.x)".to_string(), "floor(at.y)".to_string()];
	let painted = translation.call_translated(name, function, pixel, true)?;
	let functions: String = translation.functions.iter().map(|function| format!("{function}\n")).collect();
	let shader = format!(
		"{functions}@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f {{\n\treturn {painted};\n}}"
	);
	Some((shader, translation.outer))
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

fn is_assigning_op(op: Op) -> bool {
	matches!(op, Op::Assign | Op::Define) || op.is_compound_assign()
}

/// `if c then t` (`if c { t }`): c and t
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
		Node::List(items, _, _) if items.len() == 2 && items[0].is_symbol(crate::words::RETURN_WORD) => Some(&items[1]),
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
		Node::Key(target, op, _) if is_assigning_op(*op) => {
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

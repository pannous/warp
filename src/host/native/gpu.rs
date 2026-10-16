//! The GPU words natively through wgpu: gpu_compute, the @gpu maps and reductions over linear arrays, gpu_render and text_coverage
use super::*;

/// `gpu_compute(shader, numbers, workgroups)` through wgpu (src/graphics/gpu.rs): the floats the shader left
pub(super) fn gpu_compute(
	mut caller: Caller<'_, HostState>,
	shader: HostNode,
	numbers: HostNode,
	workgroups: i64,
) -> wasmtime::Result<HostNode> {
	let failure = gpu_failure(GPU_COMPUTE);
	let mut computed = || {
		let shader = given_shader(&mut caller, shader, &failure)?;
		let numbers = given_node(&mut caller, numbers)?;
		let workgroups = u32::try_from(workgroups).map_err(|_| failure(format!("{workgroups} workgroups")))?;
		match named_arrays(&numbers) {
			Some(arrays) => named_arrays_computed(&shader, arrays, workgroups, &failure),
			None => array_computed(&shader, &numbers, workgroups, &failure),
		}
	};
	let left = computed();
	let left = raised(&mut caller, left)?;
	built_in_program(&mut caller, &left, GPU_COMPUTE)
}

/// `{xs: [1 2 3], ys: [4 5 6]}`: the arrays the shader left, by name, as floats or ints as it declares them
fn named_arrays_computed(
	shader: &str,
	arrays: Vec<(String, &Node)>,
	workgroups: u32,
	failure: &impl Fn(String) -> wasmtime::Error,
) -> wasmtime::Result<Node> {
	let arrays = arrays
		.into_iter()
		.map(|(name, numbers)| {
			Ok((
				name,
				numbers
					.iter()
					.map(|number| number_of(&number, failure).map(f64::from))
					.collect::<wasmtime::Result<Vec<f64>>>()?,
			))
		})
		.collect::<wasmtime::Result<Vec<_>>>()?;
	let left = crate::graphics::gpu::compute_named(shader, &arrays, workgroups).map_err(failure)?;
	let entries = left
		.into_iter()
		.map(|(name, element, numbers)| {
			let item = |number: f64| {
				Node::Number(if element == crate::graphics::gpu::Element::Float {
					Number::Float(number)
				} else {
					Number::Int(number as i64)
				})
			};
			Node::Key(
				Box::new(Node::Symbol(name)),
				crate::operators::Op::Colon,
				Box::new(Node::List(
					numbers.into_iter().map(item).collect(),
					crate::node::Bracket::Square,
					crate::node::Separator::Space,
				)),
			)
		})
		.collect();
	Ok(Node::List(entries, crate::node::Bracket::Curly, crate::node::Separator::Space))
}

/// `[1 2 3]`: the array the shader left, of f32 or of i32/u32 as it declares its array
fn array_computed(
	shader: &str,
	numbers: &Node,
	workgroups: u32,
	failure: &impl Fn(String) -> wasmtime::Error,
) -> wasmtime::Result<Node> {
	let numbers =
		numbers.iter().map(|number| number_of(&number, failure)).collect::<wasmtime::Result<Vec<Number>>>()?;
	let left: Vec<Node> = match crate::graphics::gpu::element_of(shader) {
		crate::graphics::gpu::Element::Float => {
			let floats: Vec<f32> = numbers.iter().map(|&number| f64::from(number) as f32).collect();
			crate::graphics::gpu::compute(shader, &floats, workgroups)
				.map_err(failure)?
				.into_iter()
				.map(|float| Node::Number(Number::Float(float as f64)))
				.collect()
		}
		element => {
			let ints = numbers
				.iter()
				.map(|number| match number {
					Number::Int(int) => Ok(*int),
					other => {
						Err(failure(format!("an array<i32> or array<u32> shader computes over ints, got {other}")))
					}
				})
				.collect::<wasmtime::Result<Vec<i64>>>()?;
			crate::graphics::gpu::compute_ints(shader, &ints, workgroups, element)
				.map_err(failure)?
				.into_iter()
				.map(|int| Node::Number(Number::Int(int)))
				.collect()
		}
	};
	Ok(Node::List(left, crate::node::Bracket::Square, crate::node::Separator::Space))
}

pub(crate) fn number_of(node: &Node, failure: &impl Fn(String) -> wasmtime::Error) -> wasmtime::Result<Number> {
	match node.drop_meta() {
		Node::Number(number) => Ok(*number),
		other => Err(failure(format!("it computes over numbers, got {}", other.serialize()))),
	}
}

/// `{xs: [1 2 3], ys: [4 5 6]}`: gpu_compute's named arrays, each bound where the shader declares one of that name
pub(super) fn named_arrays(numbers: &Node) -> Option<Vec<(String, &Node)>> {
	let Node::List(entries, _, _) = numbers.drop_meta() else {
		return None;
	};
	entries
		.iter()
		.map(|entry| match entry.drop_meta() {
			Node::Key(name, _, array) => match name.drop_meta() {
				Node::Symbol(name) | Node::Text(name) => Some((name.clone(), array.as_ref())),
				_ => None,
			},
			_ => None,
		})
		.collect::<Option<Vec<_>>>()
		.filter(|arrays| !arrays.is_empty())
}

/// `gpu_compute(shader, xs, workgroups)` of a linear float array: its block `[count: i64][count f64 cells]`
/// (wasm_emitter/linear_arrays.rs) read and written in place, as f32 on the GPU
pub(super) fn gpu_compute_linear(
	mut caller: Caller<'_, HostState>,
	shader: HostNode,
	block: i64,
	workgroups: i64,
) -> wasmtime::Result<i64> {
	let failure = gpu_failure(GPU_COMPUTE);
	let shader = given_shader(&mut caller, shader, &failure)?;
	let floats = linear_floats(&mut caller, block, &failure)?;
	let workgroups = u32::try_from(workgroups).map_err(|_| failure(format!("{workgroups} workgroups")))?;
	let left = crate::graphics::gpu::compute(&shader, &floats, workgroups).map_err(&failure)?;
	write_linear_floats(&mut caller, block, &left, &failure)?;
	Ok(block)
}

/// `ys = xs.map(x => …) @gpu` (src/lowering/foreign/gpu_maps.rs): the kernel over the cells of the block `source`, then the
/// values of the program's numbers it reads, written into the block `target` of the same count; 1, or 0 without an
/// adapter (a warning, once), when the program maps on the CPU. `keeping` (gpu_maps.rs KEEP_RESULT, SOURCE_KEPT): the
/// result's buffer stays on the GPU for a later map of it, the source's items come from such a buffer
///
/// REDUCES, `s = sum(xs.map(x => …) @gpu)`, min, max: as a map, but each workgroup leaves its partial result in a cell
/// after the values, and those, the only ones read back, go into the block `target` of `workgroups` cells
pub(super) fn gpu_kernel_linear<const REDUCES: bool>(
	mut caller: Caller<'_, HostState>,
	shader: HostNode,
	source: i64,
	values: HostNode,
	target: i64,
	workgroups: i64,
	keeping: i64,
) -> wasmtime::Result<i64> {
	pub(super) static WARNED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
	let failure = gpu_failure(if REDUCES { GPU_REDUCE_LINEAR } else { GPU_MAP_LINEAR });
	let automatic = keeping & crate::gpu_maps::AUTOMATIC != 0;
	if let Err(problem) = crate::graphics::gpu::available() {
		// a map the compiler switched says nothing: the program did not ask for the GPU
		if !automatic && !WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
			crate::errors::diagnostic::report_runtime_warning(&format!("@gpu: {problem}, so the map runs on the CPU"));
		}
		return Ok(0);
	}
	let shader = given_shader(&mut caller, shader, &failure)?;
	let kept =
		(keeping & crate::gpu_maps::SOURCE_KEPT != 0).then(|| crate::graphics::gpu::kept_count(source)).flatten();
	let mut floats = if kept.is_some() { vec![] } else { linear_floats(&mut caller, source, &failure)? };
	let items = kept.unwrap_or(floats.len());
	for value in given_node(&mut caller, values)?.iter() {
		match value.drop_meta() {
			Node::Number(number) => floats.push(f64::from(*number) as f32),
			other => return Err(failure(format!("the lambda reads {}, not a number", other.serialize()))),
		}
	}
	let partials = kept.unwrap_or(0) + floats.len();
	let workgroups = u32::try_from(workgroups).map_err(|_| failure(format!("{workgroups} workgroups")))?;
	if REDUCES {
		floats.resize(floats.len() + workgroups as usize, 0.0);
	}
	let keeping = crate::graphics::gpu::Keeping {
		source: kept.map(|_| source),
		result: (keeping & crate::gpu_maps::KEEP_RESULT != 0).then_some((target, items)),
	};
	let left =
		crate::graphics::gpu::compute_kept(&shader, &floats, workgroups, if REDUCES { partials } else { 0 }, keeping)
			.map_err(&failure)?;
	write_linear_floats(&mut caller, target, &left, &failure)?;
	if automatic {
		crate::errors::diagnostic::report_runtime_warning_once(
			crate::gpu_maps::AUTOMATIC_NOTICE,
			&format!("{} of {items} items", if REDUCES { "a reduction" } else { "a map" }),
		);
	}
	Ok(1)
}

pub(super) const LINEAR_CELL_BYTES: usize = 8;

/// The byte range of the cells of a linear array's block `[count: i64][count f64 cells]` (wasm_emitter/linear_arrays.rs)
pub(super) fn linear_cells(memory: &[u8], block: i64) -> Option<std::ops::Range<usize>> {
	let start = usize::try_from(block).ok()?;
	let count = i64::from_le_bytes(memory.get(start..start + LINEAR_CELL_BYTES)?.try_into().ok()?);
	let end = start + LINEAR_CELL_BYTES + usize::try_from(count).ok()? * LINEAR_CELL_BYTES;
	(end <= memory.len()).then_some(start + LINEAR_CELL_BYTES..end)
}

pub(super) fn linear_memory(
	caller: &mut Caller<'_, HostState>,
	failure: &impl Fn(String) -> wasmtime::Error,
) -> wasmtime::Result<wasmtime::Memory> {
	match caller.get_export("memory") {
		Some(Extern::Memory(memory)) => Ok(memory),
		_ => Err(failure("the module exports no memory".into())),
	}
}

/// A linear float array's cells as f32, as the GPU takes them
pub(super) fn linear_floats(
	caller: &mut Caller<'_, HostState>,
	block: i64,
	failure: &impl Fn(String) -> wasmtime::Error,
) -> wasmtime::Result<Vec<f32>> {
	Ok(linear_doubles(caller, block, failure)?.into_iter().map(|cell| cell as f32).collect())
}

/// A linear float array's cells
pub(crate) fn linear_doubles(
	caller: &mut Caller<'_, HostState>,
	block: i64,
	failure: &impl Fn(String) -> wasmtime::Error,
) -> wasmtime::Result<Vec<f64>> {
	let memory = linear_memory(caller, failure)?;
	let bytes = memory.data(&*caller);
	let cells = linear_cells(bytes, block).ok_or_else(|| failure(format!("no linear array at {block}")))?;
	Ok(bytes[cells].as_chunks::<LINEAR_CELL_BYTES>().0.iter().map(|cell| f64::from_le_bytes(*cell)).collect())
}

/// The f32 the GPU left, written into a linear float array's cells
pub(super) fn write_linear_floats(
	caller: &mut Caller<'_, HostState>,
	block: i64,
	floats: &[f32],
	failure: &impl Fn(String) -> wasmtime::Error,
) -> wasmtime::Result<()> {
	let memory = linear_memory(caller, failure)?;
	let cells =
		linear_cells(memory.data(&*caller), block).ok_or_else(|| failure(format!("no linear array at {block}")))?;
	let written = &mut memory.data_mut(caller)[cells];
	written
		.as_chunks_mut::<LINEAR_CELL_BYTES>()
		.0
		.iter_mut()
		.zip(floats)
		.for_each(|(cell, float)| *cell = f64::from(*float).to_le_bytes());
	Ok(())
}

/// `gpu_render(shader, width, height)` through wgpu (src/graphics/gpu.rs): the pixels the fragment shader colored
pub(super) fn gpu_render(
	mut caller: Caller<'_, HostState>,
	shader: HostNode,
	width: i64,
	height: i64,
	values: HostNode,
) -> wasmtime::Result<HostNode> {
	let failure = gpu_failure(GPU_RENDER);
	let shader = given_shader(&mut caller, shader, &failure);
	let image = shader.and_then(|shader| rendered(&mut caller, &shader, width, height, values, &failure));
	let image = raised(&mut caller, image)?;
	let pixels: Vec<u32> = crate::graphics::gpu::opaque_colors(&image).collect();
	list_of_ints(&mut caller, &pixels, GPU_RENDER)
}

/// `text_coverage(words, size)` through a system font (src/graphics/text_raster.rs): [width, height, coverage row by row]
pub(super) fn text_coverage(
	mut caller: Caller<'_, HostState>,
	words: HostNode,
	size: i64,
) -> wasmtime::Result<HostNode> {
	let words = match given_node(&mut caller, words)? {
		Node::Text(text) => text,
		other => other.serialize(),
	};
	let covered = crate::graphics::text_raster::coverage(&words, size as f32)
		.map_err(|problem| task_failure(format!("{TEXT_COVERAGE}: {problem}")))?;
	list_of_ints(&mut caller, &covered, TEXT_COVERAGE)
}

/// The pixels the fragment shader colors through wgpu (src/graphics/gpu.rs), 0xFFRRGGBB row by row
pub(super) fn rendered(
	caller: &mut Caller<'_, HostState>,
	shader: &str,
	width: i64,
	height: i64,
	values: HostNode,
	failure: &impl Fn(String) -> wasmtime::Error,
) -> wasmtime::Result<Vec<u8>> {
	let (width, height, values) = shader_request(caller, width, height, values, failure)?;
	crate::graphics::gpu::render_rgba(shader, width, height, &values).map_err(failure)
}

/// A shader's image size and values as the program gave them, or why they are none
pub(super) fn shader_request(
	caller: &mut Caller<'_, HostState>,
	width: i64,
	height: i64,
	values: HostNode,
	failure: &impl Fn(String) -> wasmtime::Error,
) -> wasmtime::Result<(u32, u32, Vec<crate::graphics::gpu::ShaderValue>)> {
	let values = shader_values(&given_node(caller, values)?).map_err(failure)?;
	let side = |pixels: i64| {
		u32::try_from(pixels)
			.ok()
			.filter(|&pixels| pixels > 0)
			.ok_or_else(|| failure(format!("an image {width}×{height} pixels")))
	};
	Ok((side(width)?, side(height)?, values))
}

/// The ints as a list, built in one call of the module's ints_to_list (wasm_emitter/int_lists.rs) when it has one,
/// else node by node
pub(super) fn list_of_ints(caller: &mut Caller<'_, HostState>, ints: &[u32], word: &str) -> wasmtime::Result<HostNode> {
	use crate::wasm_emitter::lists::int_lists::INTS_TO_LIST;
	if let (Some(Extern::Func(ints_to_list)), Some(Extern::Memory(memory))) =
		(caller.get_export(INTS_TO_LIST), caller.get_export("memory"))
	{
		let ints_to_list = ints_to_list.typed::<(i32, i32), HostNode>(&*caller)?;
		if let Some(address) = scratch(caller, memory, std::mem::size_of_val(ints))? {
			let bytes: Vec<u8> = ints.iter().flat_map(|int| int.to_le_bytes()).collect();
			memory.write(&mut *caller, address as usize, &bytes)?;
			return ints_to_list.call(&mut *caller, (address as i32, ints.len() as i32));
		}
	}
	let ints = ints.iter().map(|&int| Node::Number(Number::Int(i64::from(int)))).collect();
	built_in_program(caller, &Node::List(ints, crate::node::Bracket::Square, crate::node::Separator::Space), word)
}

/// The module's memory, or the task failure "`word`: the module exports no memory"
pub(crate) fn module_memory(caller: &mut Caller<'_, HostState>, word: &str) -> wasmtime::Result<Memory> {
	match caller.get_export("memory") {
		Some(Extern::Memory(memory)) => Ok(memory),
		_ => Err(task_failure(format!("{word}: the module exports no memory"))),
	}
}

pub(super) fn gpu_failure(word: &'static str) -> impl Fn(String) -> wasmtime::Error {
	move |problem| task_failure(format!("{word}: {problem}"))
}

/// gpu_render's values (none when left out): each entry a number, a list of numbers or a list of vectors
pub(super) fn shader_values(values: &Node) -> Result<Vec<crate::graphics::gpu::ShaderValue>, String> {
	let float = |name: &str, value: &Node| match value.drop_meta() {
		Node::Number(number) => Ok(f64::from(*number) as f32),
		other => Err(format!(
			"values.{name} is a number or a list of numbers, got {}{}",
			other.serialize(),
			crate::graphics::shader_holes::missing_value_hint(name)
		)),
	};
	// a map of one entry arrives as that key
	let entries = match values.drop_meta() {
		Node::Empty => vec![],
		single @ Node::Key(..) => vec![single.clone()],
		map => map.iter().collect(),
	};
	entries
		.iter()
		.map(|entry| {
			let Node::Key(name, _, value) = entry.drop_meta() else {
				return Err(format!("values is a map of names, got {}", entry.serialize()));
			};
			let name = name.drop_meta().name();
			let floats_of = |value: &Node| match value.drop_meta() {
				Node::List(items, _, _) => {
					items.iter().map(|item| float(&name, item)).collect::<Result<Vec<f32>, String>>()
				}
				single => Ok(vec![float(&name, single)?]),
			};
			let items: Vec<Node> = match value.drop_meta() {
				Node::List(items, _, _) => items.clone(),
				_ => vec![],
			};
			// a list of lists: one vec4f per item, its missing coordinates 0
			if items.iter().any(|item| matches!(item.drop_meta(), Node::List(..))) {
				let mut floats = vec![];
				for item in &items {
					let mut vector = floats_of(item)?;
					if vector.len() > crate::graphics::gpu::VECTOR_FLOATS {
						return Err(format!(
							"values.{name} is a list of vectors of up to four numbers, got {}",
							item.serialize()
						));
					}
					vector.resize(crate::graphics::gpu::VECTOR_FLOATS, 0.0);
					floats.extend(vector);
				}
				return Ok(crate::graphics::gpu::ShaderValue { name, floats, array: true });
			}
			let floats = floats_of(value)?;
			let array = floats.len() > crate::graphics::gpu::VECTOR_FLOATS;
			Ok(crate::graphics::gpu::ShaderValue { name, floats, array })
		})
		.collect()
}

/// The WGSL text of a gpu word's shader
pub(super) fn given_shader(
	caller: &mut Caller<'_, HostState>,
	shader: HostNode,
	failure: &impl Fn(String) -> wasmtime::Error,
) -> wasmtime::Result<String> {
	match given_node(caller, shader)? {
		Node::Text(shader) => Ok(shader),
		Node::Char(letter) => Ok(letter.to_string()), // a one-letter text reads as a code point
		other => Err(failure(format!("the shader is a text of WGSL, got {}", other.serialize()))),
	}
}

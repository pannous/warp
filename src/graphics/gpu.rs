//! gpu_compute natively (card web-apis, notes/web_framework.md "web-apis: WebGPU"): the WGSL compute shader runs through
//! wgpu (Metal, Vulkan or DX12) exactly as web/playground/host-gpu.js runs it through the browser's WebGPU: entry point
//! `main`, the numbers as array<f32> (or array<i32>, array<u32>) at @group(0) @binding(0), dispatched over `workgroups` workgroups, and the value is
//! the numbers it left; a map of named arrays binds each where the shader declares the storage array of that name
//! and gives them back by name. gpu_render runs a WGSL fragment shader `main` over every pixel of a width×height image, as
//! host-gpu.js does, and gives the pixels as paint takes them.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, OnceLock};
use wgpu::util::DeviceExt;

const ENTRY_POINT: &str = "main";
const BINDING: u32 = 0;
/// `@group(0) @binding(1) var<storage, read_write> ys: array<f32>`: its binding, name and element type
const STORAGE_ARRAY: &str = r"@binding\((\d+)\)(?:\s*@group\(\d+\))?\s*var(?:<[^>]*>)?\s+(\w+)\s*:\s*array<(\w+)";
/// f32, i32 and u32 alike
const WORD_BYTES: usize = size_of::<f32>();
/// Appended to the shader of gpu_render (after it, so its line numbers stay the user's): one triangle covering the image
const FULL_IMAGE_VERTICES: &str = "
@vertex fn warp_full_image(@builtin(vertex_index) corner: u32) -> @builtin(position) vec4f {
	return vec4f(f32(corner / 2u) * 4.0 - 1.0, f32(corner % 2u) * 4.0 - 1.0, 0.0, 1.0);
}";
const VERTEX_ENTRY_POINT: &str = "warp_full_image";
const TRIANGLE_CORNERS: u32 = 3;
const PIXEL_BYTES: u32 = 4;
const OPAQUE: u32 = 0xFF00_0000;
const IMAGE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// Compiled shaders kept at once, as host-gpu.js MOST_COMPILED: a program painting more distinct ones compiles some anew
const PAINTERS_KEPT: usize = 16;
/// gpu_render's values as the shader names them: `values.<name>`
const VALUES_NAME: &str = "values";
const VALUES_BINDING: u32 = 0;
/// `$previous(at)` (card shader-read): the color the shader left at pixel `at` in its previous frame of this size, black
/// before the first; the frame is kept on the GPU, bound next to the values
const PREVIOUS_FRAME_BINDING: u32 = 1;
const PREVIOUS_FUNCTION: &str = "warp_previous";
const PREVIOUS_FRAME: &str = "
@group(0) @binding(1) var warp_previous_frame: texture_2d<f32>;
fn warp_previous(at: vec2f) -> vec4f {
	let last = vec2i(textureDimensions(warp_previous_frame)) - vec2i(1);
	return textureLoad(warp_previous_frame, clamp(vec2i(at), vec2i(0), last), 0);
}";
/// A uniform struct is a whole number of 16-byte rows
const UNIFORM_ALIGNMENT: usize = 16;

/// The floats of one vector in an array value
pub const VECTOR_FLOATS: usize = 4;

/// A value the shader reads as `values.<name>`: one to four floats (f32, vec2f, vec3f, vec4f), or an array of vec4f
/// (`values.<name>[i]`), its floats four by four
#[derive(Clone)]
pub struct ShaderValue {
	pub name: String,
	pub floats: Vec<f32>,
	pub array: bool,
}

struct Gpu {
	device: wgpu::Device,
	queue: wgpu::Queue,
}

/// The device, asked for once per process; a machine without a GPU adapter keeps its failure
fn gpu() -> Result<&'static Gpu, String> {
	static GPU: OnceLock<Result<Gpu, String>> = OnceLock::new();
	GPU.get_or_init(|| {
		let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
		let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
			.map_err(|problem| format!("no WebGPU adapter: {problem}"))?;
		let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
			.map_err(|problem| format!("no WebGPU device: {problem}"))?;
		Ok(Gpu { device, queue })
	})
	.as_ref()
	.map_err(Clone::clone)
}

/// Work with the process's device and queue (gpu_canvas.rs), or why there is none
pub fn with_device<T>(work: impl FnOnce(&wgpu::Device, &wgpu::Queue) -> T) -> Result<T, String> {
	gpu().map(|Gpu { device, queue }| work(device, queue))
}

/// Whether this machine has a GPU to run shaders on, or why not
pub fn available() -> Result<(), String> {
	gpu().map(|_| ())
}

/// Run `shader` over `numbers`: the numbers it left, or why not (no GPU, a shader that does not compile, …)
pub fn compute(shader: &str, numbers: &[f32], workgroups: u32) -> Result<Vec<f32>, String> {
	compute_from(shader, numbers, workgroups, 0)
}

/// Run `shader` over `numbers`: the numbers it left from index `first` on, the only ones read back (a reduction's
/// partial results after the items)
pub fn compute_from(shader: &str, numbers: &[f32], workgroups: u32, first: usize) -> Result<Vec<f32>, String> {
	compute_kept(shader, numbers, workgroups, first, Keeping::default())
}

/// The buffers of @gpu map results a later map starts from (gpu_maps.rs marked_kept): the result block's address, the
/// buffer holding its items first, their count. A block is never reused, so its address names one result; the oldest
/// go when more are kept
static KEPT: Mutex<Vec<(i64, wgpu::Buffer, usize)>> = Mutex::new(Vec::new());
const MOST_KEPT: usize = 4;

/// Where the items come from and where the result goes besides memory: `source`, the block whose kept buffer holds
/// the items (before `numbers`); `result`, the block to keep the buffer for and its count of items
#[derive(Default)]
pub struct Keeping {
	pub source: Option<i64>,
	pub result: Option<(i64, usize)>,
}

/// The count of items the GPU keeps for the block, if it does
pub fn kept_count(block: i64) -> Option<usize> {
	kept_buffer(block).map(|(_, count)| count)
}

fn kept_buffer(block: i64) -> Option<(wgpu::Buffer, usize)> {
	KEPT.lock().ok()?.iter().find(|(kept, _, _)| *kept == block).map(|(_, buffer, count)| (buffer.clone(), *count))
}

/// As compute_from, the items taken from a kept buffer and the result's buffer kept, as `keeping` says
pub fn compute_kept(
	shader: &str,
	numbers: &[f32],
	workgroups: u32,
	first: usize,
	keeping: Keeping,
) -> Result<Vec<f32>, String> {
	let bytes: Vec<u8> = numbers.iter().flat_map(|number| number.to_le_bytes()).collect();
	Ok(words(&compute_bytes(shader, &bytes, workgroups, first, keeping)?).map(f32::from_le_bytes).collect())
}

/// The element type of the numbers a compute shader reads at @binding(0): f32 unless it declares array<i32> or array<u32>
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Element {
	Float,
	Int,
	Unsigned,
}

pub fn element_of(shader: &str) -> Element {
	storage_arrays(shader)
		.into_iter()
		.find(|array| array.binding == BINDING)
		.map_or(Element::Float, |array| array.element)
}

/// Run a shader over 32-bit ints (array<i32> or array<u32>, as `element` says): the ints it left
pub fn compute_ints(shader: &str, numbers: &[i64], workgroups: u32, element: Element) -> Result<Vec<i64>, String> {
	let numbers: Vec<f64> = numbers.iter().map(|&number| number as f64).collect();
	let left = compute_bytes(shader, &encoded(&numbers, element)?, workgroups, 0, Keeping::default())?;
	Ok(decoded(&left, element).into_iter().map(|number| number as i64).collect())
}

/// The numbers as the shader's 4-byte elements: f32, or i32 / u32, which must hold them
fn encoded(numbers: &[f64], element: Element) -> Result<Vec<u8>, String> {
	numbers
		.iter()
		.map(|&number| {
			let whole = number.fract() == 0.0;
			match element {
				Element::Float => Ok((number as f32).to_le_bytes()),
				Element::Int if whole && (i32::MIN as f64..=i32::MAX as f64).contains(&number) => {
					Ok((number as i32).to_le_bytes())
				}
				Element::Unsigned if whole && (0.0..=u32::MAX as f64).contains(&number) => {
					Ok((number as u32).to_le_bytes())
				}
				Element::Int => Err(format!("{number} is no i32")),
				Element::Unsigned => Err(format!("{number} is no u32")),
			}
		})
		.collect::<Result<Vec<_>, _>>()
		.map(|words| words.concat())
}

fn decoded(bytes: &[u8], element: Element) -> Vec<f64> {
	words(bytes)
		.map(|word| match element {
			Element::Float => f32::from_le_bytes(word) as f64,
			Element::Int => i32::from_le_bytes(word) as f64,
			Element::Unsigned => u32::from_le_bytes(word) as f64,
		})
		.collect()
}

/// A storage array the shader declares: `@group(0) @binding(1) var<storage, read_write> ys: array<f32>;`
pub struct StorageArray {
	pub binding: u32,
	pub name: String,
	pub element: Element,
}

pub fn storage_arrays(shader: &str) -> Vec<StorageArray> {
	static DECLARATION: LazyLock<regex::Regex> =
		LazyLock::new(|| regex::Regex::new(STORAGE_ARRAY).expect("the pattern of a storage array"));
	DECLARATION
		.captures_iter(shader)
		.map(|declared| StorageArray {
			binding: declared[1].parse().expect("digits"),
			name: declared[2].to_string(),
			element: match &declared[3] {
				"i32" => Element::Int,
				"u32" => Element::Unsigned,
				_ => Element::Float,
			},
		})
		.collect()
}

/// `gpu_compute(shader, {xs: […], ys: […]}, workgroups)`: each named array bound where the shader declares the
/// storage array of that name; the arrays it left, in the order given, as their element type says
pub fn compute_named(
	shader: &str,
	arrays: &[(String, Vec<f64>)],
	workgroups: u32,
) -> Result<Vec<(String, Element, Vec<f64>)>, String> {
	let declared = storage_arrays(shader);
	let names = || declared.iter().map(|array| array.name.as_str()).collect::<Vec<_>>().join(", ");
	if let Some(missing) = declared.iter().find(|array| !arrays.iter().any(|(name, _)| *name == array.name)) {
		return Err(format!("the shader's array {} has no numbers: give each of {}", missing.name, names()));
	}
	let buffers = arrays
		.iter()
		.map(|(name, numbers)| {
			let array = declared
				.iter()
				.find(|array| array.name == *name)
				.ok_or_else(|| format!("the shader declares no storage array {name}, only {}", names()))?;
			if numbers.is_empty() {
				return Err(format!("{name} is empty: a storage array holds at least one number"));
			}
			Ok((array.binding, array.element, encoded(numbers, array.element)?))
		})
		.collect::<Result<Vec<_>, String>>()?;
	let left = compute_buffers(
		shader,
		&buffers.iter().map(|(binding, _, bytes)| (*binding, bytes.as_slice())).collect::<Vec<_>>(),
		workgroups,
	)?;
	Ok(arrays
		.iter()
		.zip(&buffers)
		.zip(left)
		.map(|(((name, _), (_, element, _)), bytes)| (name.clone(), *element, decoded(&bytes, *element)))
		.collect())
}

/// The shader over a storage buffer per binding: the bytes each holds afterwards
fn compute_buffers(shader: &str, buffers: &[(u32, &[u8])], workgroups: u32) -> Result<Vec<Vec<u8>>, String> {
	let Gpu { device, queue } = gpu()?;
	let usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST;
	let storages: Vec<wgpu::Buffer> = buffers
		.iter()
		.map(|(_, bytes)| {
			device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("gpu_compute array"),
				contents: bytes,
				usage,
			})
		})
		.collect();
	let readbacks: Vec<wgpu::Buffer> = storages.iter().map(|storage| readback_of(device, storage.size())).collect();
	let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
	let pipeline = compute_pipeline(device, shader)?;
	let entries: Vec<wgpu::BindGroupEntry> = buffers
		.iter()
		.zip(&storages)
		.map(|((binding, _), storage)| wgpu::BindGroupEntry {
			binding: *binding,
			resource: storage.as_entire_binding(),
		})
		.collect();
	let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
		label: Some("gpu_compute arrays"),
		layout: &pipeline.get_bind_group_layout(0),
		entries: &entries,
	});
	let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
	dispatch(&mut encoder, &pipeline, &bindings, workgroups);
	for (storage, readback) in storages.iter().zip(&readbacks) {
		encoder.copy_buffer_to_buffer(storage, 0, readback, 0, storage.size());
	}
	queue.submit([encoder.finish()]);
	read_backs(device, &readbacks, validation)
}

fn readback_of(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
	device.create_buffer(&wgpu::BufferDescriptor {
		label: Some("gpu_compute readback"),
		size,
		usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
		mapped_at_creation: false,
	})
}

/// The compute pipeline of the shader's entry point `main`, its bind group laid out as the shader declares it
fn compute_pipeline(device: &wgpu::Device, shader: &str) -> Result<wgpu::ComputePipeline, String> {
	let module = compiled(device, shader)?;
	Ok(device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
		label: Some("gpu_compute"),
		layout: None,
		module: &module,
		entry_point: Some(ENTRY_POINT),
		compilation_options: Default::default(),
		cache: None,
	}))
}

fn dispatch(
	encoder: &mut wgpu::CommandEncoder,
	pipeline: &wgpu::ComputePipeline,
	bindings: &wgpu::BindGroup,
	workgroups: u32,
) {
	let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
	pass.set_pipeline(pipeline);
	pass.set_bind_group(0, bindings, &[]);
	pass.dispatch_workgroups(workgroups, 1, 1);
}

fn words(bytes: &[u8]) -> impl Iterator<Item = [u8; 4]> + '_ {
	bytes.as_chunks::<WORD_BYTES>().0.iter().copied()
}

/// The shader over the 4-byte numbers in `bytes`: the bytes it left from number `first` on
fn compute_bytes(
	shader: &str,
	bytes: &[u8],
	workgroups: u32,
	first: usize,
	keeping: Keeping,
) -> Result<Vec<u8>, String> {
	let Gpu { device, queue } = gpu()?;
	let kept = keeping.source.and_then(kept_buffer);
	let items = kept.as_ref().map_or(0, |(_, count)| *count);
	let total = items + bytes.len() / WORD_BYTES;
	let offset = (first.min(total) * WORD_BYTES) as u64;
	let size = (total * WORD_BYTES) as u64 - offset;
	let usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST;
	let storage = device.create_buffer(&wgpu::BufferDescriptor {
		label: Some("gpu_compute numbers"),
		size: (total * WORD_BYTES) as u64,
		usage,
		mapped_at_creation: false,
	});
	if !bytes.is_empty() {
		queue.write_buffer(&storage, (items * WORD_BYTES) as u64, bytes);
	}
	let readback = readback_of(device, size);

	let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
	let pipeline = compute_pipeline(device, shader)?;
	let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
		label: Some("gpu_compute numbers"),
		layout: &pipeline.get_bind_group_layout(0),
		entries: &[wgpu::BindGroupEntry { binding: BINDING, resource: storage.as_entire_binding() }],
	});
	let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
	if let Some((buffer, count)) = &kept {
		encoder.copy_buffer_to_buffer(buffer, 0, &storage, 0, (count * WORD_BYTES) as u64);
	}
	dispatch(&mut encoder, &pipeline, &bindings, workgroups);
	encoder.copy_buffer_to_buffer(&storage, offset, &readback, 0, size);
	queue.submit([encoder.finish()]);
	let bytes = read_back(device, &readback, validation)?;
	if let (Some((block, count)), Ok(mut kept)) = (keeping.result, KEPT.lock()) {
		// an earlier run's buffer for the same block address is stale
		kept.retain(|(earlier, _, _)| *earlier != block);
		if kept.len() >= MOST_KEPT {
			kept.remove(0);
		}
		kept.push((block, storage, count));
	}
	Ok(bytes)
}

/// Run the fragment shader `main` over a width×height image: its pixels row by row as 0xFFRRGGBB (alpha dropped:
/// paint shows opaque colors), or why not
pub fn render(shader: &str, width: u32, height: u32, values: &[ShaderValue]) -> Result<Vec<u32>, String> {
	Ok(opaque_colors(&render_rgba(shader, width, height, values)?).collect())
}

/// The pixels of RGBA bytes as 0xFFRRGGBB
pub fn opaque_colors(rgba: &[u8]) -> impl Iterator<Item = u32> + '_ {
	let pixels = rgba.as_chunks::<{ PIXEL_BYTES as usize }>().0;
	pixels
		.iter()
		.map(|[red, green, blue, _]| OPAQUE | u32::from(*red) << 16 | u32::from(*green) << 8 | u32::from(*blue))
}

/// The image of `render` as RGBA bytes, row by row: what paint's window takes without a per-pixel pass (a frame of the
/// debug build costs 55 ms less at 1280×720)
pub fn render_rgba(shader: &str, width: u32, height: u32, values: &[ShaderValue]) -> Result<Vec<u8>, String> {
	let values = &with_builtin_values(shader, width, height, values);
	let Gpu { device, queue } = gpu()?;
	let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
	let (image, encoder) = drawn_image(device, shader, width, height, values, None)?;
	checked_image_rgba(device, queue, encoder, &image, validation)
}

/// The values a paint window renders the shader with (card paint-keeps): the program's and the built-in ones it reads
/// ($time, $mouse, …), once the shader compiles here; why not, before anything reaches the window
pub fn checked_values(
	shader: &str,
	width: u32,
	height: u32,
	values: &[ShaderValue],
) -> Result<Vec<ShaderValue>, String> {
	let values = with_builtin_values(shader, width, height, values);
	let Gpu { device, .. } = gpu()?;
	let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
	let prepared = prepared(device, shader, &values);
	if let Some(invalid) = pollster::block_on(validation.pop()) {
		return Err(essence(&invalid.to_string()));
	}
	prepared.map(|_| values)
}

/// The shader rendered with the values (checked_values) into a texture on `device` that stays there: what the paint
/// window shows, viewed as `view_format` when it differs (the sRGB view of a surface)
pub fn render_texture(
	device: &wgpu::Device,
	queue: &wgpu::Queue,
	shader: &str,
	width: u32,
	height: u32,
	values: &[ShaderValue],
	view_format: Option<wgpu::TextureFormat>,
) -> Result<wgpu::Texture, String> {
	let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
	let (image, encoder) = drawn_image(device, shader, width, height, values, view_format)?;
	queue.submit([encoder.finish()]);
	match pollster::block_on(validation.pop()) {
		Some(invalid) => Err(essence(&invalid.to_string())),
		None => Ok(image),
	}
}

/// The shader with its values compiled into its painter, and the values' bytes
fn prepared(device: &wgpu::Device, shader: &str, values: &[ShaderValue]) -> Result<(Painter, Vec<u8>), String> {
	let (declarations, bytes) = uniform_layout(values)?;
	let reads_previous = reads_value(shader, crate::graphics::shader_holes::PREVIOUS_HOLE);
	let source = match reads_previous {
		true => format!(
			"{}{FULL_IMAGE_VERTICES}{declarations}{PREVIOUS_FRAME}",
			shader
				.replace(&format!("{VALUES_NAME}.{}", crate::graphics::shader_holes::PREVIOUS_HOLE), PREVIOUS_FUNCTION)
		),
		false => format!("{shader}{FULL_IMAGE_VERTICES}{declarations}"),
	};
	let painter = painter(device, source, !bytes.is_empty(), reads_previous)?;
	Ok((painter, bytes))
}

/// The image the shader draws, and the encoder holding the drawing
fn drawn_image(
	device: &wgpu::Device,
	shader: &str,
	width: u32,
	height: u32,
	values: &[ShaderValue],
	view_format: Option<wgpu::TextureFormat>,
) -> Result<(wgpu::Texture, wgpu::CommandEncoder), String> {
	let (Painter { pipeline, bindings_layout }, bytes) = prepared(device, shader, values)?;
	let reads_previous = reads_value(shader, crate::graphics::shader_holes::PREVIOUS_HOLE);
	let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
	let previous = reads_previous.then(|| previous_frame(device, shader, size));
	let bindings = bindings_layout.map(|layout| bindings(device, &layout, &bytes, previous.as_ref()));
	let usage =
		wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::TEXTURE_BINDING;
	let view_formats: Vec<_> = view_format.into_iter().filter(|format| *format != IMAGE_FORMAT).collect();
	let image = device.create_texture(&wgpu::TextureDescriptor {
		label: Some("gpu_render image"),
		size,
		mip_level_count: 1,
		sample_count: 1,
		dimension: wgpu::TextureDimension::D2,
		format: IMAGE_FORMAT,
		usage,
		view_formats: &view_formats,
	});
	if reads_previous {
		keep_previous_frame(shader, image.clone());
	}
	let view = image.create_view(&wgpu::TextureViewDescriptor::default());
	let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
	{
		let target = wgpu::RenderPassColorAttachment {
			view: &view,
			depth_slice: None,
			resolve_target: None,
			ops: wgpu::Operations::default(),
		};
		let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("gpu_render"),
			color_attachments: &[Some(target)],
			..Default::default()
		});
		pass.set_pipeline(&pipeline);
		if let Some(bindings) = &bindings {
			pass.set_bind_group(0, bindings, &[]);
		}
		pass.draw(0..TRIANGLE_CORNERS, 0..1);
	}
	Ok((image, encoder))
}

/// The RGBA bytes of a 4-byte-per-pixel image, row by row, once the encoder's work on it is done
pub fn image_rgba(
	device: &wgpu::Device,
	queue: &wgpu::Queue,
	encoder: wgpu::CommandEncoder,
	image: &wgpu::Texture,
) -> Result<Vec<u8>, String> {
	checked_image_rgba(device, queue, encoder, image, device.push_error_scope(wgpu::ErrorFilter::Validation))
}

fn checked_image_rgba(
	device: &wgpu::Device,
	queue: &wgpu::Queue,
	mut encoder: wgpu::CommandEncoder,
	image: &wgpu::Texture,
	validation: wgpu::ErrorScopeGuard,
) -> Result<Vec<u8>, String> {
	let size = image.size();
	let (width, height) = (size.width, size.height);
	// a copied row is padded to 256 bytes
	let row_bytes = (width * PIXEL_BYTES).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
	let readback = device.create_buffer(&wgpu::BufferDescriptor {
		label: Some("image readback"),
		size: u64::from(row_bytes * height),
		usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
		mapped_at_creation: false,
	});
	let layout =
		wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row_bytes), rows_per_image: Some(height) };
	encoder.copy_texture_to_buffer(
		image.as_image_copy(),
		wgpu::TexelCopyBufferInfo { buffer: &readback, layout },
		size,
	);
	queue.submit([encoder.finish()]);
	let bytes = read_back(device, &readback, validation)?;
	let mut rgba = Vec::with_capacity((width * height * PIXEL_BYTES) as usize);
	for row in bytes.chunks_exact(row_bytes as usize) {
		rgba.extend_from_slice(&row[..(width * PIXEL_BYTES) as usize]);
	}
	Ok(rgba)
}

/// The frames shaders reading $previous left, by shader text: each render draws into a new image and keeps it
static PREVIOUS_FRAMES: LazyLock<Mutex<HashMap<String, wgpu::Texture>>> = LazyLock::new(Default::default);

/// The view of the frame the shader left last, a black one when it left none of this size
fn previous_frame(device: &wgpu::Device, shader: &str, size: wgpu::Extent3d) -> wgpu::TextureView {
	let kept = PREVIOUS_FRAMES.lock().unwrap().get(shader).filter(|frame| frame.size() == size).cloned();
	let frame = kept.unwrap_or_else(|| {
		device.create_texture(&wgpu::TextureDescriptor {
			label: Some("gpu_render previous frame"),
			size,
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: IMAGE_FORMAT,
			usage: wgpu::TextureUsages::TEXTURE_BINDING,
			view_formats: &[],
		})
	});
	frame.create_view(&wgpu::TextureViewDescriptor::default())
}

fn keep_previous_frame(shader: &str, frame: wgpu::Texture) {
	let mut frames = PREVIOUS_FRAMES.lock().unwrap();
	if frames.len() >= PAINTERS_KEPT && !frames.contains_key(shader) {
		frames.clear();
	}
	frames.insert(shader.to_string(), frame);
}

/// The values with the built-in holes the shader reads and the program gave no value (shader_holes.rs BUILTIN_HOLES)
fn with_builtin_values(shader: &str, width: u32, height: u32, values: &[ShaderValue]) -> Vec<ShaderValue> {
	static FIRST_RENDER: LazyLock<std::time::Instant> = LazyLock::new(std::time::Instant::now);
	static RENDERS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
	let frame = RENDERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed) as f32;
	let size = [width as f32, height as f32];
	let [mouse_x, mouse_y, mouse_down, key] = warp_runtime::system_values::window_input();
	let filled = crate::graphics::shader_holes::BUILTIN_HOLES
		.into_iter()
		.filter(|name| *name != crate::graphics::shader_holes::PREVIOUS_HOLE)
		.filter(|name| reads_value(shader, name) && !values.iter().any(|value| value.name == *name));
	let builtins = filled.map(|name| ShaderValue {
		name: name.to_string(),
		floats: match name {
			"width" => vec![size[0]],
			"height" => vec![size[1]],
			"size" => size.to_vec(),
			"time" => vec![FIRST_RENDER.elapsed().as_secs_f32()],
			"frame" => vec![frame],
			"mouse" => vec![mouse_x, mouse_y],
			"mouse_down" => vec![mouse_down],
			"key" => vec![key],
			other => unreachable!("{other} is no built-in hole"),
		},
		array: false,
	});
	values.iter().cloned().chain(builtins).collect()
}

/// Whether the shader reads `values.name`
fn reads_value(shader: &str, name: &str) -> bool {
	let read = format!("{}.{name}", crate::graphics::shader_holes::VALUES);
	shader
		.match_indices(&read)
		.any(|(at, _)| !shader[at + read.len()..].starts_with(|ch: char| ch.is_alphanumeric() || ch == '_'))
}

/// The WGSL declaring `values` (appended to the shader, so its line numbers stay the user's) and the bytes of the
/// uniform, laid out as WGSL aligns its members: f32 by 4, vec2f by 8, vec3f, vec4f and arrays of vec4f by 16
pub fn uniform_layout(values: &[ShaderValue]) -> Result<(String, Vec<u8>), String> {
	if values.is_empty() {
		return Ok((String::new(), Vec::new()));
	}
	let mut members = Vec::new();
	let mut bytes = Vec::new();
	for ShaderValue { name, floats, array } in values {
		let (wgsl_type, alignment) = match floats.len() {
			_ if *array => (format!("array<vec4f, {}>", floats.len().div_ceil(VECTOR_FLOATS).max(1)), 16),
			1 => ("f32".into(), 4),
			2 => ("vec2f".into(), 8),
			3 => ("vec3f".into(), 16),
			4 => ("vec4f".into(), 16),
			count => return Err(format!("{VALUES_NAME}.{name} is one number or a list of two to four, got {count}")),
		};
		bytes.resize(bytes.len().next_multiple_of(alignment), 0);
		bytes.extend(floats.iter().flat_map(|float| float.to_le_bytes()));
		if *array {
			bytes.resize(bytes.len().next_multiple_of(UNIFORM_ALIGNMENT), 0);
		}
		members.push(format!("{name}: {wgsl_type}"));
	}
	bytes.resize(bytes.len().next_multiple_of(UNIFORM_ALIGNMENT), 0);
	let declarations = format!(
		"\nstruct WarpValues {{ {} }}\n@group(0) @binding({VALUES_BINDING}) var<uniform> {VALUES_NAME}: WarpValues;",
		members.join(", ")
	);
	Ok((declarations, bytes))
}

/// A fragment shader compiled into its pipeline, with the layout of its values and previous frame (none: it takes
/// neither)
#[derive(Clone)]
struct Painter {
	pipeline: wgpu::RenderPipeline,
	bindings_layout: Option<wgpu::BindGroupLayout>,
}

/// The painter of a shader text, compiled once: compiling a pipeline anew for every frame costs more than drawing it
/// (over 100 ms for a raymarching shader on Metal, samples/gpu/gpu_visualizer.warp)
fn painter(device: &wgpu::Device, source: String, takes_values: bool, reads_previous: bool) -> Result<Painter, String> {
	static PAINTERS: LazyLock<Mutex<HashMap<String, Painter>>> = LazyLock::new(Default::default);
	let mut painters = PAINTERS.lock().unwrap();
	if let Some(painter) = painters.get(&source) {
		return Ok(painter.clone());
	}
	let module = compiled(device, &source)?;
	// explicit, so a shader that leaves `values` unread still binds them
	let values = takes_values.then_some(wgpu::BindGroupLayoutEntry {
		binding: VALUES_BINDING,
		visibility: wgpu::ShaderStages::FRAGMENT,
		ty: wgpu::BindingType::Buffer {
			ty: wgpu::BufferBindingType::Uniform,
			has_dynamic_offset: false,
			min_binding_size: None,
		},
		count: None,
	});
	let previous = reads_previous.then_some(wgpu::BindGroupLayoutEntry {
		binding: PREVIOUS_FRAME_BINDING,
		visibility: wgpu::ShaderStages::FRAGMENT,
		ty: wgpu::BindingType::Texture {
			sample_type: wgpu::TextureSampleType::Float { filterable: false },
			view_dimension: wgpu::TextureViewDimension::D2,
			multisampled: false,
		},
		count: None,
	});
	let entries: Vec<_> = values.into_iter().chain(previous).collect();
	let bindings_layout = (!entries.is_empty()).then(|| {
		device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: Some("gpu_render values"),
			entries: &entries,
		})
	});
	let layout = bindings_layout.as_ref().map(|values| {
		device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("gpu_render"),
			bind_group_layouts: &[Some(values)],
			immediate_size: 0,
		})
	});
	let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: Some("gpu_render"),
		layout: layout.as_ref(),
		vertex: wgpu::VertexState {
			module: &module,
			entry_point: Some(VERTEX_ENTRY_POINT),
			compilation_options: Default::default(),
			buffers: &[],
		},
		primitive: wgpu::PrimitiveState::default(),
		depth_stencil: None,
		multisample: wgpu::MultisampleState::default(),
		fragment: Some(wgpu::FragmentState {
			module: &module,
			entry_point: Some(ENTRY_POINT),
			compilation_options: Default::default(),
			targets: &[Some(IMAGE_FORMAT.into())],
		}),
		multiview_mask: None,
		cache: None,
	});
	if painters.len() >= PAINTERS_KEPT {
		painters.clear();
	}
	let painter = Painter { pipeline, bindings_layout };
	painters.insert(source, painter.clone());
	Ok(painter)
}

/// This frame's values bound to the painter's layout
/// The values' bytes (none: the shader takes none) and the previous frame, bound as `layout` says
fn bindings(
	device: &wgpu::Device,
	layout: &wgpu::BindGroupLayout,
	bytes: &[u8],
	previous: Option<&wgpu::TextureView>,
) -> wgpu::BindGroup {
	let buffer = (!bytes.is_empty()).then(|| {
		device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("gpu_render values"),
			contents: bytes,
			usage: wgpu::BufferUsages::UNIFORM,
		})
	});
	let values = buffer
		.as_ref()
		.map(|buffer| wgpu::BindGroupEntry { binding: VALUES_BINDING, resource: buffer.as_entire_binding() });
	let previous = previous.map(|view| wgpu::BindGroupEntry {
		binding: PREVIOUS_FRAME_BINDING,
		resource: wgpu::BindingResource::TextureView(view),
	});
	let entries: Vec<_> = values.into_iter().chain(previous).collect();
	device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("gpu_render values"), layout, entries: &entries })
}

/// The shader's module, or where and why it does not compile
fn compiled(device: &wgpu::Device, shader: &str) -> Result<wgpu::ShaderModule, String> {
	let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
		label: Some("warp shader"),
		source: wgpu::ShaderSource::Wgsl(shader.into()),
	});
	let problems = compile_errors(&module);
	if problems.is_empty() {
		Ok(module)
	} else {
		Err(problems.join("; "))
	}
}

/// The bytes the submitted work left in `readback`, or the validation error the work raised
fn read_back(
	device: &wgpu::Device,
	readback: &wgpu::Buffer,
	validation: wgpu::ErrorScopeGuard,
) -> Result<Vec<u8>, String> {
	Ok(read_backs(device, std::slice::from_ref(readback), validation)?.concat())
}

/// The bytes the submitted work left in each readback buffer, or the validation error the work raised
fn read_backs(
	device: &wgpu::Device,
	readbacks: &[wgpu::Buffer],
	validation: wgpu::ErrorScopeGuard,
) -> Result<Vec<Vec<u8>>, String> {
	if let Some(invalid) = pollster::block_on(validation.pop()) {
		return Err(essence(&invalid.to_string()));
	}
	for readback in readbacks {
		readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
	}
	device.poll(wgpu::PollType::wait_indefinitely()).map_err(|problem| problem.to_string())?;
	readbacks
		.iter()
		.map(|readback| {
			readback.slice(..).get_mapped_range().map(|view| view.to_vec()).map_err(|problem| problem.to_string())
		})
		.collect()
}

/// `1:10: expected identifier, found "{"`: where and why the shader does not compile, as the browser says it
fn compile_errors(module: &wgpu::ShaderModule) -> Vec<String> {
	let errors = pollster::block_on(module.get_compilation_info())
		.messages
		.into_iter()
		.filter(|message| message.message_type == wgpu::CompilationMessageType::Error);
	errors
		.map(|error| match error.location {
			Some(at) => format!("{}:{}: {}", at.line_number, at.line_position, essence(&error.message)),
			None => essence(&error.message),
		})
		.collect()
}

/// wgpu's report in one line: `parsing error: expected identifier` gives what follows `error: `, a nested report its
/// innermost cause (`Unable to find entry point 'main'`)
fn essence(report: &str) -> String {
	let lines = || report.lines().map(str::trim).filter(|line| !line.is_empty());
	let cause = lines().find_map(|line| line.split_once("error: ").map(|(_, why)| why)).or_else(|| lines().next_back());
	cause.unwrap_or(report).to_string()
}

//! gpu_compute natively (card web-apis, notes/web_framework.md "web-apis: WebGPU"): the WGSL compute shader runs through
//! wgpu (Metal, Vulkan or DX12) exactly as web/playground/host-gpu.js runs it through the browser's WebGPU: entry point
//! `main`, the numbers as array<f32> at @group(0) @binding(0), dispatched over `workgroups` workgroups, and the value is
//! the numbers it left. gpu_render runs a WGSL fragment shader `main` over every pixel of a width×height image, as
//! host-gpu.js does, and gives the pixels as paint takes them.

use std::sync::OnceLock;
use wgpu::util::DeviceExt;

const ENTRY_POINT: &str = "main";
const BINDING: u32 = 0;
const FLOAT_BYTES: usize = size_of::<f32>();
/// Appended to the shader of gpu_render (after it, so its line numbers stay the user's): one triangle covering the image
const FULL_IMAGE_VERTICES: &str = "
@vertex fn warp_full_image(@builtin(vertex_index) corner: u32) -> @builtin(position) vec4f {
	return vec4f(f32(corner / 2u) * 4.0 - 1.0, f32(corner % 2u) * 4.0 - 1.0, 0.0, 1.0);
}";
const VERTEX_ENTRY_POINT: &str = "warp_full_image";
const TRIANGLE_CORNERS: u32 = 3;
const PIXEL_BYTES: u32 = 4;
const OPAQUE: u32 = 0xFF00_0000;

struct Gpu {
	device: wgpu::Device,
	queue: wgpu::Queue,
}

/// The device, asked for once per process; a machine without a GPU adapter keeps its failure
fn gpu() -> Result<&'static Gpu, String> {
	static GPU: OnceLock<Result<Gpu, String>> = OnceLock::new();
	GPU.get_or_init(|| {
		let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
		let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).map_err(|problem| format!("no WebGPU adapter: {problem}"))?;
		let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).map_err(|problem| format!("no WebGPU device: {problem}"))?;
		Ok(Gpu { device, queue })
	})
	.as_ref()
	.map_err(Clone::clone)
}

/// Run `shader` over `numbers`: the numbers it left, or why not (no GPU, a shader that does not compile, …)
pub fn compute(shader: &str, numbers: &[f32], workgroups: u32) -> Result<Vec<f32>, String> {
	let Gpu { device, queue } = gpu()?;
	let bytes: Vec<u8> = numbers.iter().flat_map(|number| number.to_le_bytes()).collect();
	let size = bytes.len() as u64;
	let usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST;
	let storage = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("gpu_compute numbers"), contents: &bytes, usage });
	let readback = device.create_buffer(&wgpu::BufferDescriptor { label: Some("gpu_compute readback"), size, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });

	let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
	let module = compiled(device, shader)?;
	let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
		label: Some("gpu_compute"),
		layout: None,
		module: &module,
		entry_point: Some(ENTRY_POINT),
		compilation_options: Default::default(),
		cache: None,
	});
	let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
		label: Some("gpu_compute numbers"),
		layout: &pipeline.get_bind_group_layout(0),
		entries: &[wgpu::BindGroupEntry { binding: BINDING, resource: storage.as_entire_binding() }],
	});
	let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
	{
		let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
		pass.set_pipeline(&pipeline);
		pass.set_bind_group(0, &bindings, &[]);
		pass.dispatch_workgroups(workgroups, 1, 1);
	}
	encoder.copy_buffer_to_buffer(&storage, 0, &readback, 0, size);
	queue.submit([encoder.finish()]);
	let bytes = read_back(device, &readback, validation)?;
	Ok(bytes.chunks_exact(FLOAT_BYTES).map(|chunk| f32::from_le_bytes(chunk.try_into().expect("four bytes"))).collect())
}

/// Run the fragment shader `main` over a width×height image: its pixels row by row as 0xFFRRGGBB (alpha dropped:
/// paint shows opaque colors), or why not
pub fn render(shader: &str, width: u32, height: u32) -> Result<Vec<u32>, String> {
	let Gpu { device, queue } = gpu()?;
	let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
	let module = compiled(device, &format!("{shader}{FULL_IMAGE_VERTICES}"))?;
	let format = wgpu::TextureFormat::Rgba8Unorm;
	let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
	let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC;
	let image = device.create_texture(&wgpu::TextureDescriptor { label: Some("gpu_render image"), size, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format, usage, view_formats: &[] });
	// a copied row is padded to 256 bytes
	let row_bytes = (width * PIXEL_BYTES).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
	let readback = device.create_buffer(&wgpu::BufferDescriptor { label: Some("gpu_render readback"), size: u64::from(row_bytes * height), usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
	let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: Some("gpu_render"),
		layout: None,
		vertex: wgpu::VertexState { module: &module, entry_point: Some(VERTEX_ENTRY_POINT), compilation_options: Default::default(), buffers: &[] },
		primitive: wgpu::PrimitiveState::default(),
		depth_stencil: None,
		multisample: wgpu::MultisampleState::default(),
		fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some(ENTRY_POINT), compilation_options: Default::default(), targets: &[Some(format.into())] }),
		multiview_mask: None,
		cache: None,
	});
	let view = image.create_view(&wgpu::TextureViewDescriptor::default());
	let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
	{
		let target = wgpu::RenderPassColorAttachment { view: &view, depth_slice: None, resolve_target: None, ops: wgpu::Operations::default() };
		let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { label: Some("gpu_render"), color_attachments: &[Some(target)], ..Default::default() });
		pass.set_pipeline(&pipeline);
		pass.draw(0..TRIANGLE_CORNERS, 0..1);
	}
	let layout = wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row_bytes), rows_per_image: Some(height) };
	encoder.copy_texture_to_buffer(image.as_image_copy(), wgpu::TexelCopyBufferInfo { buffer: &readback, layout }, size);
	queue.submit([encoder.finish()]);
	let bytes = read_back(device, &readback, validation)?;
	let rows = bytes.chunks_exact(row_bytes as usize).map(|row| &row[..(width * PIXEL_BYTES) as usize]);
	Ok(rows.flat_map(|row| row.chunks_exact(PIXEL_BYTES as usize).map(|rgba| OPAQUE | u32::from(rgba[0]) << 16 | u32::from(rgba[1]) << 8 | u32::from(rgba[2]))).collect())
}

/// The shader's module, or where and why it does not compile
fn compiled(device: &wgpu::Device, shader: &str) -> Result<wgpu::ShaderModule, String> {
	let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("warp shader"), source: wgpu::ShaderSource::Wgsl(shader.into()) });
	let problems = compile_errors(&module);
	if problems.is_empty() { Ok(module) } else { Err(problems.join("; ")) }
}

/// The bytes the submitted work left in `readback`, or the validation error the work raised
fn read_back(device: &wgpu::Device, readback: &wgpu::Buffer, validation: wgpu::ErrorScopeGuard) -> Result<Vec<u8>, String> {
	if let Some(invalid) = pollster::block_on(validation.pop()) {
		return Err(essence(&invalid.to_string()));
	}
	readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
	device.poll(wgpu::PollType::wait_indefinitely()).map_err(|problem| problem.to_string())?;
	let view = readback.slice(..).get_mapped_range().map_err(|problem| problem.to_string())?;
	Ok(view.to_vec())
}

/// `1:10: expected identifier, found "{"`: where and why the shader does not compile, as the browser says it
fn compile_errors(module: &wgpu::ShaderModule) -> Vec<String> {
	let errors = pollster::block_on(module.get_compilation_info()).messages.into_iter().filter(|message| message.message_type == wgpu::CompilationMessageType::Error);
	errors.map(|error| match error.location {
		Some(at) => format!("{}:{}: {}", at.line_number, at.line_position, essence(&error.message)),
		None => essence(&error.message),
	}).collect()
}

/// wgpu's report in one line: `parsing error: expected identifier` gives what follows `error: `, a nested report its
/// innermost cause (`Unable to find entry point 'main'`)
fn essence(report: &str) -> String {
	let lines = || report.lines().map(str::trim).filter(|line| !line.is_empty());
	let cause = lines().find_map(|line| line.split_once("error: ").map(|(_, why)| why)).or_else(|| lines().next_back());
	cause.unwrap_or(report).to_string()
}

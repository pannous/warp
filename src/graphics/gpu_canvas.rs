//! `use draw` on the GPU (card gpu-draw): clear, dot, rect and circle of lib/draw.warp become shapes, 24 bytes each,
//! which lib/draw.warp's shape_shader (draw_shader) draws into a canvas texture that stays on the GPU; show() sends only the shapes drawn since the last
//! show, never the pixels. In a window the paint-window process owns the texture (src/graphics/paint_window.rs); headless
//! (WARP_DRAW=gpu without a window) an offscreen one here, read back for paint's PNG. The CPU draw of lib/draw.warp
//! stays for headless runs and the playground; the system value gpu_drawing tells lib/draw.warp which one runs.
//! A shape covers the pixels whose centers the CPU draw would color, so both give the same picture: a circle the
//! centers within its radius, a rect the cells from cell(x) to cell(x + width); alpha mixes as CPU's mixed() does.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// draw_shape's kind of clear (lib/draw.warp clear_shape), from which on the shapes cover all
pub const CLEAR: u32 = 0;
/// kind u32, color u32 (0xAARRGGBB), four f32 coordinates: rect x y width height, circle x y radius
pub const SHAPE_BYTES: usize = 24;
/// The canvas before anything is drawn: paint's paper (pixels.rs, lib/draw.warp paper)
const PAPER_LEVEL: f64 = super::pixels::PAPER as f64 / 255.0;
const PAPER: wgpu::Color = wgpu::Color { r: PAPER_LEVEL, g: PAPER_LEVEL, b: PAPER_LEVEL, a: 1.0 };
/// Bytes, mixed as bytes: CPU's mixed() mixes 0–255 channels, not light
const CANVAS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const LABEL: &str = "draw canvas";
/// Two triangles: the box a shape covers
const BOX_CORNERS: u32 = 6;
const CANVAS_SIZE_BYTES: u64 = 16;
/// draw_shader(code) of lib/draw.warp: the shader drawing the shapes (its kinds as draw_shape gives them), in this
/// process and in the paint window's, which reads it first (paint_window::shape_shader_message)
static SHAPE_SHADER: Mutex<Option<String>> = Mutex::new(None);
/// Whether the paint window has the shape shader
static SHADER_SENT: AtomicBool = AtomicBool::new(false);

/// The shapes drawn since the last show, as they cross to the GPU
static SHAPES: Mutex<Vec<u8>> = Mutex::new(Vec::new());
/// The shapes not yet drawn into OFFSCREEN, which keeps the canvas here too: a window's canvas lives in its process,
/// so a saved picture (snapshot, card save-painting) is drawn here from them. From the last clear on, which covers all
static UNMIRRORED: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// draw_shape(kind, a, b, c, d, color): one more shape for the next show
pub fn add_shape(kind: u32, place: [f64; 4], color: u32) {
	let mut shape = [kind.to_le_bytes(), color.to_le_bytes()].concat();
	for coordinate in place {
		shape.extend((coordinate as f32).to_le_bytes());
	}
	SHAPES.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).extend(&shape);
	let mut unmirrored = UNMIRRORED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	if kind == CLEAR {
		unmirrored.clear();
	}
	unmirrored.extend(shape);
}

/// draw_shader(code): the shape shader lib/draw.warp writes
pub fn use_shader(code: String) {
	*SHAPE_SHADER.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(code);
}

fn shape_shader() -> Result<String, String> {
	let shader = SHAPE_SHADER.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone();
	shader.ok_or_else(|| "use draw on the GPU: lib/draw.warp gave no shape shader (draw_shader)".to_string())
}

/// The shapes since the last show, taken
pub fn take_shapes() -> Vec<u8> {
	std::mem::take(&mut *SHAPES.lock().unwrap_or_else(|poisoned| poisoned.into_inner()))
}

/// The canvas texture and the pipeline drawing shapes into it
pub struct Canvas {
	pipeline: wgpu::RenderPipeline,
	size_buffer: wgpu::Buffer,
	bindings: wgpu::BindGroup,
	texture: Option<wgpu::Texture>,
}

impl Canvas {
	pub fn new(device: &wgpu::Device) -> Result<Canvas, String> {
		let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: Some(LABEL),
			source: wgpu::ShaderSource::Wgsl(shape_shader()?.into()),
		});
		let attributes = wgpu::vertex_attr_array![0 => Uint32x2, 1 => Float32x4];
		let shapes = wgpu::VertexBufferLayout {
			array_stride: SHAPE_BYTES as u64,
			step_mode: wgpu::VertexStepMode::Instance,
			attributes: &attributes,
		};
		let target = wgpu::ColorTargetState {
			format: CANVAS_FORMAT,
			blend: Some(wgpu::BlendState::ALPHA_BLENDING),
			write_mask: wgpu::ColorWrites::ALL,
		};
		let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some(LABEL),
			layout: None,
			vertex: wgpu::VertexState {
				module: &module,
				entry_point: Some("vertex"),
				compilation_options: Default::default(),
				buffers: &[Some(shapes)],
			},
			fragment: Some(wgpu::FragmentState {
				module: &module,
				entry_point: Some("fragment"),
				compilation_options: Default::default(),
				targets: &[Some(target)],
			}),
			primitive: Default::default(),
			depth_stencil: None,
			multisample: Default::default(),
			multiview_mask: None,
			cache: None,
		});
		let size_buffer = device.create_buffer(&wgpu::BufferDescriptor {
			label: Some(LABEL),
			size: CANVAS_SIZE_BYTES,
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});
		let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some(LABEL),
			layout: &pipeline.get_bind_group_layout(0),
			entries: &[wgpu::BindGroupEntry { binding: 0, resource: size_buffer.as_entire_binding() }],
		});
		Ok(Canvas { pipeline, size_buffer, bindings, texture: None })
	}

	/// The shapes drawn over the canvas, a new one of paper when the size changed; the encoder still to submit
	pub fn draw(
		&mut self,
		device: &wgpu::Device,
		queue: &wgpu::Queue,
		width: u32,
		height: u32,
		shapes: &[u8],
	) -> wgpu::CommandEncoder {
		let size = wgpu::Extent3d { width: width.max(1), height: height.max(1), depth_or_array_layers: 1 };
		let fresh = self.texture.as_ref().is_none_or(|texture| texture.size() != size);
		if fresh {
			let usage = wgpu::TextureUsages::RENDER_ATTACHMENT
				| wgpu::TextureUsages::TEXTURE_BINDING
				| wgpu::TextureUsages::COPY_SRC;
			let view_formats = [CANVAS_FORMAT.add_srgb_suffix()];
			self.texture = Some(device.create_texture(&wgpu::TextureDescriptor {
				label: Some(LABEL),
				size,
				mip_level_count: 1,
				sample_count: 1,
				dimension: wgpu::TextureDimension::D2,
				format: CANVAS_FORMAT,
				usage,
				view_formats: &view_formats,
			}));
			let sides = [size.width as f32, size.height as f32, 0.0, 0.0];
			queue.write_buffer(
				&self.size_buffer,
				0,
				&sides.iter().flat_map(|side| side.to_le_bytes()).collect::<Vec<u8>>(),
			);
		}
		let mut encoder = device.create_command_encoder(&Default::default());
		let view = self.texture().create_view(&Default::default());
		let count = (shapes.len() / SHAPE_BYTES) as u32;
		let shape_buffer = (count > 0).then(|| {
			let buffer = device.create_buffer(&wgpu::BufferDescriptor {
				label: Some(LABEL),
				size: shapes.len() as u64,
				usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
				mapped_at_creation: false,
			});
			queue.write_buffer(&buffer, 0, shapes);
			buffer
		});
		let load = if fresh { wgpu::LoadOp::Clear(PAPER) } else { wgpu::LoadOp::Load };
		let attachment = wgpu::RenderPassColorAttachment {
			view: &view,
			depth_slice: None,
			resolve_target: None,
			ops: wgpu::Operations { load, store: wgpu::StoreOp::Store },
		};
		let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some(LABEL),
			color_attachments: &[Some(attachment)],
			..Default::default()
		});
		if let Some(buffer) = &shape_buffer {
			pass.set_pipeline(&self.pipeline);
			pass.set_bind_group(0, &self.bindings, &[]);
			pass.set_vertex_buffer(0, buffer.slice(..));
			pass.draw(0..BOX_CORNERS, 0..count);
		}
		drop(pass);
		encoder
	}

	pub fn texture(&self) -> &wgpu::Texture {
		self.texture.as_ref().expect("the canvas is drawn before it is read")
	}
}

/// The canvas headless, kept from show to show as the window keeps its own
static OFFSCREEN: Mutex<Option<Canvas>> = Mutex::new(None);

/// show() of `use draw` on the GPU: the new shapes to the paint window, or headless drawn here and painted as RGBA
pub fn show(width: usize, height: usize) -> Result<Option<std::path::PathBuf>, String> {
	let shapes = take_shapes();
	if crate::graphics::paint::shows_windows() {
		let mut message = match SHADER_SENT.swap(true, Ordering::Relaxed) {
			true => vec![],
			false => crate::graphics::paint_window::shape_shader_message(&shape_shader()?),
		};
		message.extend(crate::graphics::paint_window::shapes_message(width, height, &shapes));
		crate::graphics::paint_window::show(&message)?;
		return Ok(None);
	}
	crate::graphics::paint::paint_rgba(&snapshot(width, height)?, width, height)
}

/// The canvas as drawn so far, its RGBA bytes: the shapes not drawn here yet drawn into OFFSCREEN, which is read back
pub fn snapshot(width: usize, height: usize) -> Result<Vec<u8>, String> {
	let shapes = std::mem::take(&mut *UNMIRRORED.lock().unwrap_or_else(|poisoned| poisoned.into_inner()));
	crate::graphics::gpu::with_device(|device, queue| {
		let mut offscreen = OFFSCREEN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
		if offscreen.is_none() {
			*offscreen = Some(Canvas::new(device)?);
		}
		let canvas = offscreen.as_mut().expect("made above");
		let encoder = canvas.draw(device, queue, width as u32, height as u32, &shapes);
		crate::graphics::gpu::image_rgba(device, queue, encoder, canvas.texture())
	})?
}

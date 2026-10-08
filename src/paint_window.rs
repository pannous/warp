//! paint(pixels, width, height) in a terminal (card g_gGsg): the frames in a window, drawn through a wgpu surface (Metal
//! on macOS). A window owns the main thread there, which the program runs on, so a viewer process does: the same
//! binary as `warp paint-window`, reading frames from its stdin, [width u32][height u32][width×height RGBA bytes]. Each
//! paint call replaces the frame (samples/webgpu.warp animates); the last one stays until the window is closed.

use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

pub const COMMAND: &str = "paint-window";
/// `warp paint-window --check`: the first frame drawn is read back from the window's surface, its center pixel printed
/// as `center r g b`, and the viewer ends (probes/paint_window.sh: what reaches the screen, without a screenshot)
pub const CHECK: &str = "--check";
/// A copy to a buffer takes whole rows of this many bytes
const COPY_ROW_BYTES: u32 = 256;
const TITLE: &str = "warp paint";
/// A small image is scaled up by a whole factor until its longer side reaches this many points
const SHOWN_SIDE: u32 = 512;
const RGBA_BYTES: usize = 4;
/// One triangle covering the window, sampling the frame as a texture, nearest pixel (no blur when scaled up)
const SHADER: &str = "
@group(0) @binding(0) var frame: texture_2d<f32>;
@group(0) @binding(1) var nearest: sampler;
struct Corner { @builtin(position) at: vec4f, @location(0) uv: vec2f }
@vertex fn vertex(@builtin(vertex_index) corner: u32) -> Corner {
	let xy = vec2f(f32(corner / 2u) * 4.0 - 1.0, f32(corner % 2u) * 4.0 - 1.0);
	return Corner(vec4f(xy, 0.0, 1.0), vec2f(xy.x + 1.0, 1.0 - xy.y) * 0.5);
}
@fragment fn fragment(corner: Corner) -> @location(0) vec4f {
	return textureSample(frame, nearest, corner.uv);
}";
const TRIANGLE_CORNERS: u32 = 3;

/// A painted image as the viewer takes it
#[derive(Debug, PartialEq)]
pub struct Frame {
	pub width: u32,
	pub height: u32,
	pub rgba: Vec<u8>,
}

/// The pixels (paint's values, shaded as the PNG shades them) as one frame for the viewer's stdin
pub fn frame_bytes(pixels: &[u64], width: usize, height: usize) -> Vec<u8> {
	let mut bytes = Vec::with_capacity(8 + width * height * RGBA_BYTES);
	bytes.extend((width as u32).to_le_bytes());
	bytes.extend((height as u32).to_le_bytes());
	for &value in &pixels[..width * height] {
		bytes.extend(crate::paint::shade(value));
		bytes.push(u8::MAX);
	}
	bytes
}

/// The next frame, None at the end of the stream
pub fn read_frame(reader: &mut impl Read) -> Option<Frame> {
	let mut size = [0u8; 8];
	reader.read_exact(&mut size).ok()?;
	let side = |at: usize| u32::from_le_bytes(size[at..at + 4].try_into().expect("four bytes"));
	let (width, height) = (side(0), side(4));
	let mut rgba = vec![0u8; width as usize * height as usize * RGBA_BYTES];
	reader.read_exact(&mut rgba).ok()?;
	Some(Frame { width, height, rgba })
}

/// The viewer of this run, started by the first paint
static VIEWER: Mutex<Option<Child>> = Mutex::new(None);

/// Show the pixels in the viewer's window, or why not (no viewer: the caller writes a PNG instead)
pub fn show(pixels: &[u64], width: usize, height: usize) -> Result<(), String> {
	let mut viewer = VIEWER.lock().map_err(|_| "paint: the viewer lock is poisoned".to_string())?;
	if viewer.is_none() {
		let binary = std::env::current_exe().map_err(|failure| format!("paint: no viewer, the warp binary is unknown: {failure}"))?;
		let mut command = Command::new(binary);
		command.arg(COMMAND).stdin(Stdio::piped()).stdout(Stdio::null());
		// its own process group: the window outlives the program and the terminal's hangup when that closes
		#[cfg(unix)]
		std::os::unix::process::CommandExt::process_group(&mut command, 0);
		let child = command.spawn();
		*viewer = Some(child.map_err(|failure| format!("paint: cannot start the viewer: {failure}"))?);
	}
	let input = viewer.as_mut().and_then(|child| child.stdin.as_mut()).expect("the viewer's stdin is piped");
	let written = input.write_all(&frame_bytes(pixels, width, height)).and_then(|_| input.flush());
	written.map_err(|failure| {
		*viewer = None;
		format!("paint: the viewer is gone ({failure})")
	})
}

/// `warp paint-window`: the window showing the frames arriving on stdin, until it is closed
pub fn run(check: bool) -> Result<(), String> {
	let event_loop = EventLoop::<Frame>::with_user_event().build().map_err(|failure| format!("paint-window: no event loop: {failure}"))?;
	let proxy = event_loop.create_proxy();
	std::thread::spawn(move || {
		let mut input = std::io::stdin().lock();
		while let Some(frame) = read_frame(&mut input) {
			if proxy.send_event(frame).is_err() {
				break;
			}
		}
	});
	let mut viewer = Viewer { check, ..Viewer::default() };
	event_loop.run_app(&mut viewer).map_err(|failure| format!("paint-window: {failure}"))?;
	viewer.failure.map_or(Ok(()), Err)
}

#[derive(Default)]
struct Viewer {
	screen: Option<Screen>,
	/// The frame that came before the window could take it
	waiting: Option<Frame>,
	failure: Option<String>,
	check: bool,
}

/// The window and what draws into it
struct Screen {
	window: Arc<Window>,
	surface: wgpu::Surface<'static>,
	config: wgpu::SurfaceConfiguration,
	device: wgpu::Device,
	queue: wgpu::Queue,
	pipeline: wgpu::RenderPipeline,
	sampler: wgpu::Sampler,
	texture_format: wgpu::TextureFormat,
	bindings: Option<wgpu::BindGroup>,
	frame_size: (u32, u32),
}

impl ApplicationHandler<Frame> for Viewer {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		if self.screen.is_some() {
			return;
		}
		let (width, height) = self.waiting.as_ref().map_or((SHOWN_SIDE, SHOWN_SIDE), |frame| (frame.width, frame.height));
		match Screen::open(event_loop, width, height, self.check) {
			Ok(screen) => self.screen = Some(screen),
			Err(failure) => {
				self.failure = Some(failure);
				return event_loop.exit();
			}
		}
		if let Some(frame) = self.waiting.take() {
			self.user_event(event_loop, frame);
		}
	}

	fn user_event(&mut self, _: &ActiveEventLoop, frame: Frame) {
		match self.screen.as_mut() {
			Some(screen) => screen.take(frame),
			None => self.waiting = Some(frame),
		}
	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
		let Some(screen) = self.screen.as_mut() else { return };
		match event {
			WindowEvent::CloseRequested => event_loop.exit(),
			WindowEvent::Resized(size) => screen.resize(size.width, size.height),
			WindowEvent::RedrawRequested => {
				if let Some(center) = screen.draw(self.check) {
					println!("center {center}");
					event_loop.exit();
				}
			}
			_ => {}
		}
	}
}

impl Screen {
	fn open(event_loop: &ActiveEventLoop, width: u32, height: u32, check: bool) -> Result<Screen, String> {
		let scale = (SHOWN_SIDE / width.max(height).max(1)).max(1);
		let attributes = Window::default_attributes().with_title(TITLE).with_inner_size(winit::dpi::LogicalSize::new(width * scale, height * scale));
		let window = Arc::new(event_loop.create_window(attributes).map_err(|failure| format!("paint-window: no window: {failure}"))?);
		let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
		let surface = instance.create_surface(window.clone()).map_err(|failure| format!("paint-window: no surface: {failure}"))?;
		let options = wgpu::RequestAdapterOptions { compatible_surface: Some(&surface), ..Default::default() };
		let adapter = pollster::block_on(instance.request_adapter(&options)).map_err(|problem| format!("paint-window: no GPU adapter: {problem}"))?;
		let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).map_err(|problem| format!("paint-window: no GPU device: {problem}"))?;
		let size = window.inner_size();
		let mut config = surface.get_default_config(&adapter, size.width.max(1), size.height.max(1)).ok_or("paint-window: the surface does not fit the GPU")?;
		if check {
			config.usage |= wgpu::TextureUsages::COPY_SRC;
		}
		surface.configure(&device, &config);
		// the frame's bytes are sRGB like the surface's: sampled linear, written back as sRGB
		let texture_format = if config.format.is_srgb() { wgpu::TextureFormat::Rgba8UnormSrgb } else { wgpu::TextureFormat::Rgba8Unorm };
		let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some(TITLE), source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
		let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some(TITLE),
			layout: None,
			vertex: wgpu::VertexState { module: &module, entry_point: Some("vertex"), compilation_options: Default::default(), buffers: &[] },
			fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some("fragment"), compilation_options: Default::default(), targets: &[Some(config.format.into())] }),
			primitive: Default::default(),
			depth_stencil: None,
			multisample: Default::default(),
			multiview_mask: None,
			cache: None,
		});
		let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
		Ok(Screen { window, surface, config, device, queue, pipeline, sampler, texture_format, bindings: None, frame_size: (0, 0) })
	}

	/// The frame as the texture the next draw samples
	fn take(&mut self, frame: Frame) {
		let size = wgpu::Extent3d { width: frame.width.max(1), height: frame.height.max(1), depth_or_array_layers: 1 };
		let texture = self.device.create_texture(&wgpu::TextureDescriptor {
			label: Some(TITLE),
			size,
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: self.texture_format,
			usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
			view_formats: &[],
		});
		if frame.width > 0 && frame.height > 0 {
			let layout = wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(frame.width * RGBA_BYTES as u32), rows_per_image: None };
			self.queue.write_texture(texture.as_image_copy(), &frame.rgba, layout, size);
		}
		let view = texture.create_view(&Default::default());
		self.bindings = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some(TITLE),
			layout: &self.pipeline.get_bind_group_layout(0),
			entries: &[
				wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
				wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
			],
		}));
		if self.frame_size != (frame.width, frame.height) && self.frame_size != (0, 0) {
			let scale = (SHOWN_SIDE / frame.width.max(frame.height).max(1)).max(1);
			let _ = self.window.request_inner_size(winit::dpi::LogicalSize::new(frame.width * scale, frame.height * scale));
		}
		self.frame_size = (frame.width, frame.height);
		self.window.request_redraw();
	}

	fn resize(&mut self, width: u32, height: u32) {
		if width > 0 && height > 0 {
			(self.config.width, self.config.height) = (width, height);
			self.surface.configure(&self.device, &self.config);
			self.window.request_redraw();
		}
	}

	/// Draw the frame; with `check`, the center pixel of what was drawn, once there is a frame
	fn draw(&mut self, check: bool) -> Option<String> {
		let output = match self.surface.get_current_texture() {
			wgpu::CurrentSurfaceTexture::Success(texture) | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
			_ => {
				self.surface.configure(&self.device, &self.config);
				self.window.request_redraw();
				return None;
			}
		};
		let target = output.texture.create_view(&Default::default());
		let mut encoder = self.device.create_command_encoder(&Default::default());
		{
			let attachment = wgpu::RenderPassColorAttachment { view: &target, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store } };
			let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[Some(attachment)], ..Default::default() });
			if let Some(bindings) = &self.bindings {
				pass.set_pipeline(&self.pipeline);
				pass.set_bind_group(0, bindings, &[]);
				pass.draw(0..TRIANGLE_CORNERS, 0..1);
			}
		}
		let readback = (check && self.bindings.is_some()).then(|| self.copied_center(&mut encoder, &output.texture));
		self.queue.submit([encoder.finish()]);
		let center = readback.map(|buffer| self.center_of(&buffer));
		self.window.pre_present_notify();
		self.queue.present(output);
		center
	}

	/// The center pixel of `texture` copied into a buffer
	fn copied_center(&self, encoder: &mut wgpu::CommandEncoder, texture: &wgpu::Texture) -> wgpu::Buffer {
		let buffer = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some(TITLE), size: COPY_ROW_BYTES.into(), usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
		let mut center = texture.as_image_copy();
		center.origin = wgpu::Origin3d { x: self.config.width / 2, y: self.config.height / 2, z: 0 };
		let layout = wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(COPY_ROW_BYTES), rows_per_image: None };
		encoder.copy_texture_to_buffer(center, wgpu::TexelCopyBufferInfo { buffer: &buffer, layout }, wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 });
		buffer
	}

	/// `r g b` of the copied pixel, whatever the order of the surface's format
	fn center_of(&self, buffer: &wgpu::Buffer) -> String {
		buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
		if let Err(problem) = self.device.poll(wgpu::PollType::wait_indefinitely()) {
			return format!("unread: {problem}");
		}
		let Ok(bytes) = buffer.slice(..).get_mapped_range() else { return "unread".into() };
		let blue_first = matches!(self.config.format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
		let [red, green, blue] = if blue_first { [bytes[2], bytes[1], bytes[0]] } else { [bytes[0], bytes[1], bytes[2]] };
		format!("{red} {green} {blue}")
	}
}

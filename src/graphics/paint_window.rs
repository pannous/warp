//! paint(pixels, width, height) in a terminal (card g_gGsg): the frames in a window, drawn through a wgpu surface (Metal
//! on macOS). A window owns the main thread there, which the program runs on, so a viewer process does: the same
//! binary as `warp paint-window`, reading frames from its stdin, [width u32][height u32][width×height RGBA bytes]. Each
//! paint call replaces the frame (samples/graphics/webgpu.warp animates); the last one stays until the window is closed, the
//! program waiting for that (wait_until_closed). The program ending otherwise ends its stdin and so the viewer.
//! `use draw` on the GPU sends shapes instead (src/graphics/gpu_canvas.rs): [SHAPES_MARK][width][height][count][count shapes],
//! drawn over the canvas texture this process keeps, which the window then shows; no pixels cross.
//! A shader painted in a window (card paint-keeps) sends itself and its values:
//! [SHADER_MARK][width][height][shader bytes][value count][shader][values], each value [name bytes][float count][array
//! 0 or 1][name][floats as f32]; this process renders it on its own GPU device, no frame of pixels crosses.
//! `window.width = 800` (card make-width) sends [RESIZE_MARK][width][height] in points: the window takes that size and
//! keeps it, its frames scaled into it. The viewer tells its size as `size width height` (points) as it opens and resizes.

use std::io::{BufRead, Read, Write};
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
/// A frame this many pixels across or more shows pixel for pixel (crisp on a Retina screen, half as many points)
const PIXEL_FOR_PIXEL_SIDE: u32 = 1024;
const RGBA_BYTES: usize = 4;
/// In a frame's width: shapes follow, not pixels
const SHAPES_MARK: u32 = u32::MAX;
/// In a frame's width: a shader and its values follow
const SHADER_MARK: u32 = u32::MAX - 1;
/// In a frame's width: the window's size the program asks for follows
const RESIZE_MARK: u32 = u32::MAX - 2;
const WORD_BYTES: usize = 4;
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
	return vec4f(textureSample(frame, nearest, corner.uv).rgb, 1.0); // a shader's alpha: paint shows opaque colors
}";
const TRIANGLE_CORNERS: u32 = 3;
/// The line the viewer writes to its stdout on the pointer and the keys, and with its first frame (the pointer in its
/// middle then): `input x y down key held…`, the pointer in the frame's pixels, down 1 while a button is, key as
/// shader_holes.rs BUILTIN_HOLES says, then the code of every key held, in the order they went down (card allow-diagonals)
const INPUT_LINE: &str = "input";
/// The line the viewer writes as its window opens or resizes: `size width height`, in points
const SIZE_LINE: &str = "size";
/// The line the viewer writes as the user ends it (closing the window, Cmd-Q): the run ends with it (card
/// escape-window), its sounds stopped; so does Escape, unless the program names that key (system_values escape_kept)
const CLOSED_LINE: &str = "closed";
/// Escape's code, as key_code gives it
const ESCAPE_CODE: f32 = 27.0;
/// The arrows' codes, as a Mac's function keys: up, down, left, right
const ARROW_CODES: [u32; 4] = [0xF700, 0xF701, 0xF702, 0xF703];
/// NSTrackingArea's options: mouse moves, whether the window is key or not, over the whole view as it resizes
pub const TRACKING_OPTIONS: usize = TRACKING_MOUSE_MOVED | TRACKING_ACTIVE_ALWAYS | TRACKING_IN_VISIBLE_RECT;
const TRACKING_MOUSE_MOVED: usize = 0x02;
pub const TRACKING_ACTIVE_ALWAYS: usize = 0x80;
const TRACKING_IN_VISIBLE_RECT: usize = 0x200;

/// The viewer's input lines, read until it ends; whether the user ended it
pub fn follow_input(lines: impl BufRead) -> bool {
	for line in lines.lines().map_while(Result::ok) {
		let mut words = line.split_whitespace();
		let kind = words.next();
		let numbers: Vec<f32> = words.map_while(|word| word.parse().ok()).collect();
		match (kind, numbers.as_slice()) {
			(Some(INPUT_LINE), &[_, _, _, key, ..])
				if key == ESCAPE_CODE && !warp_runtime::system_values::escape_kept() =>
			{
				return true
			}
			(Some(INPUT_LINE), &[x, y, down, key, ref held @ ..]) => {
				let held: Vec<i64> = held.iter().map(|code| *code as i64).collect();
				warp_runtime::system_values::tell_keys_held(&held);
				warp_runtime::system_values::tell_window_input([x, y, down, key]);
			}
			(Some(SIZE_LINE), &[width, height]) => {
				warp_runtime::system_values::tell_window_size(width as i64, height as i64)
			}
			(Some(CLOSED_LINE), []) => return true,
			_ => {}
		}
	}
	false
}

/// The user ended the window: so ends the run, its sounds stopped, else the process's end would wait for them
fn end_the_run() -> ! {
	crate::sound::stop();
	let _ = std::io::stdout().flush();
	std::process::exit(0)
}

/// The code of a pressed key: its character's, an arrow's ARROW_CODES, Enter, Escape, Backspace and Tab their control
/// characters' (lowering/signals/system_signals.rs KEY_NAMES names them); None for the others
fn key_code(key: &winit::keyboard::Key) -> Option<u32> {
	use winit::keyboard::{Key, NamedKey};
	match key {
		Key::Character(text) => text.chars().next().map(u32::from),
		Key::Named(NamedKey::ArrowUp) => Some(ARROW_CODES[0]),
		Key::Named(NamedKey::ArrowDown) => Some(ARROW_CODES[1]),
		Key::Named(NamedKey::ArrowLeft) => Some(ARROW_CODES[2]),
		Key::Named(NamedKey::ArrowRight) => Some(ARROW_CODES[3]),
		Key::Named(NamedKey::Space) => Some(u32::from(' ')),
		Key::Named(NamedKey::Enter) => Some(u32::from('\r')),
		Key::Named(NamedKey::Escape) => Some(0x1B),
		Key::Named(NamedKey::Backspace) => Some(0x08),
		Key::Named(NamedKey::Tab) => Some(u32::from('\t')),
		_ => None,
	}
}

/// A painted image as the viewer takes it
#[derive(Debug, PartialEq)]
pub struct Frame {
	pub width: u32,
	pub height: u32,
	pub rgba: Vec<u8>,
}

/// Shapes for the canvas `use draw` keeps on the GPU
#[derive(Debug, PartialEq)]
pub struct Shapes {
	pub width: u32,
	pub height: u32,
	pub shapes: Vec<u8>,
}

/// A shader for the viewer to render, with the values it reads (gpu::checked_values)
pub struct ShaderFrame {
	pub width: u32,
	pub height: u32,
	pub shader: String,
	pub values: Vec<crate::graphics::gpu::ShaderValue>,
}

/// What the viewer reads from its stdin
pub enum Message {
	Frame(Frame),
	Shapes(Shapes),
	Shader(ShaderFrame),
	/// The window's size in points the program asks for
	Resize(u32, u32),
}

impl Message {
	fn size(&self) -> (u32, u32) {
		match self {
			Message::Frame(frame) => (frame.width, frame.height),
			Message::Shapes(shapes) => (shapes.width, shapes.height),
			Message::Shader(shader) => (shader.width, shader.height),
			Message::Resize(width, height) => (*width, *height),
		}
	}
}

/// The pixels (paint's values, shaded as the PNG shades them) as one frame for the viewer's stdin
pub fn frame_bytes(pixels: &[u64], width: usize, height: usize) -> Vec<u8> {
	let mut bytes = frame_header(width, height);
	for &value in &pixels[..width * height] {
		bytes.extend(crate::graphics::paint::shade(value));
		bytes.push(u8::MAX);
	}
	bytes
}

/// RGBA bytes (a rendered shader, gpu::render_rgba) as one frame for the viewer's stdin
pub fn rgba_frame(rgba: &[u8], width: usize, height: usize) -> Vec<u8> {
	let mut bytes = frame_header(width, height);
	bytes.extend_from_slice(&rgba[..width * height * RGBA_BYTES]);
	bytes
}

fn frame_header(width: usize, height: usize) -> Vec<u8> {
	let mut bytes = Vec::with_capacity(8 + width * height * RGBA_BYTES);
	bytes.extend((width as u32).to_le_bytes());
	bytes.extend((height as u32).to_le_bytes());
	bytes
}

/// The shapes drawn since the last show as one message for the viewer's stdin
pub fn shapes_message(width: usize, height: usize, shapes: &[u8]) -> Vec<u8> {
	let count = shapes.len() / crate::graphics::gpu_canvas::SHAPE_BYTES;
	let mut bytes = words_bytes(&[SHAPES_MARK, width as u32, height as u32, count as u32]);
	bytes.extend(shapes);
	bytes
}

/// The shader and its values as one message for the viewer's stdin
pub fn shader_message(width: u32, height: u32, shader: &str, values: &[crate::graphics::gpu::ShaderValue]) -> Vec<u8> {
	let mut bytes = words_bytes(&[SHADER_MARK, width, height, shader.len() as u32, values.len() as u32]);
	bytes.extend(shader.as_bytes());
	for value in values {
		bytes.extend(words_bytes(&[value.name.len() as u32, value.floats.len() as u32, u32::from(value.array)]));
		bytes.extend(value.name.as_bytes());
		bytes.extend(value.floats.iter().flat_map(|float| float.to_le_bytes()));
	}
	bytes
}

fn words_bytes(words: &[u32]) -> Vec<u8> {
	words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

fn read_shader_frame(reader: &mut impl Read, width: u32) -> Option<ShaderFrame> {
	let [height, shader_bytes, count] = read_words(reader)?;
	let shader = String::from_utf8(read_bytes(reader, shader_bytes as usize)?).ok()?;
	let values = (0..count)
		.map(|_| {
			let [name_bytes, floats, array] = read_words(reader)?;
			let name = String::from_utf8(read_bytes(reader, name_bytes as usize)?).ok()?;
			let floats = read_bytes(reader, floats as usize * WORD_BYTES)?;
			let floats = floats.as_chunks::<WORD_BYTES>().0.iter().map(|bytes| f32::from_le_bytes(*bytes)).collect();
			Some(crate::graphics::gpu::ShaderValue { name, floats, array: array != 0 })
		})
		.collect::<Option<_>>()?;
	Some(ShaderFrame { width, height, shader, values })
}

/// The next frame, None at the end of the stream (or at shapes or a shader)
pub fn read_frame(reader: &mut impl Read) -> Option<Frame> {
	match read_message(reader)? {
		Message::Frame(frame) => Some(frame),
		_ => None,
	}
}

/// The next frame or shapes, None at the end of the stream
pub fn read_message(reader: &mut impl Read) -> Option<Message> {
	let [first, second] = read_words(reader)?;
	if first == SHADER_MARK {
		return read_shader_frame(reader, second).map(Message::Shader);
	}
	if first == RESIZE_MARK {
		let [height] = read_words(reader)?;
		return Some(Message::Resize(second, height));
	}
	if first == SHAPES_MARK {
		let [height, count] = read_words(reader)?;
		let shapes = read_bytes(reader, count as usize * crate::graphics::gpu_canvas::SHAPE_BYTES)?;
		return Some(Message::Shapes(Shapes { width: second, height, shapes }));
	}
	let rgba = read_bytes(reader, first as usize * second as usize * RGBA_BYTES)?;
	Some(Message::Frame(Frame { width: first, height: second, rgba }))
}

fn read_words<const COUNT: usize>(reader: &mut impl Read) -> Option<[u32; COUNT]> {
	let bytes = read_bytes(reader, COUNT * WORD_BYTES)?;
	Some(std::array::from_fn(|at| {
		u32::from_le_bytes(bytes[at * WORD_BYTES..][..WORD_BYTES].try_into().expect("four bytes"))
	}))
}

fn read_bytes(reader: &mut impl Read, count: usize) -> Option<Vec<u8>> {
	let mut bytes = vec![0u8; count];
	reader.read_exact(&mut bytes).ok()?;
	Some(bytes)
}

/// The viewer of this run, started by the first paint
static VIEWER: Mutex<Option<Child>> = Mutex::new(None);
/// The window's size the program asked for (window.width = …), sent to a viewer as it starts
static ASKED_SIZE: Mutex<Option<(u32, u32)>> = Mutex::new(None);

/// [RESIZE_MARK][width][height]: the window's size in points
pub fn resize_message(width: u32, height: u32) -> Vec<u8> {
	words_bytes(&[RESIZE_MARK, width, height])
}

/// `window.width = 800`: the window's size in points, now if it shows, else as it opens
pub fn resize(width: i64, height: i64) {
	let (width, height) = (width.clamp(1, u32::MAX as i64) as u32, height.clamp(1, u32::MAX as i64) as u32);
	if !VIEWER.lock().is_ok_and(|viewer| viewer.is_some()) {
		*ASKED_SIZE.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some((width, height));
	} else if let Err(failure) = show(&resize_message(width, height)) {
		eprintln!("window.width: {failure}");
	}
}

/// Show the pixels in the viewer's window, or why not (no viewer: the caller writes a PNG instead)
pub fn show(frame: &[u8]) -> Result<(), String> {
	let mut viewer = VIEWER.lock().map_err(|_| "paint: the viewer lock is poisoned".to_string())?;
	if viewer.is_none() {
		let binary = std::env::current_exe()
			.map_err(|failure| format!("paint: no viewer, the warp binary is unknown: {failure}"))?;
		let mut command = Command::new(binary);
		command.arg(COMMAND).stdin(Stdio::piped()).stdout(Stdio::piped());
		// its own process group: ctrl-c reaches the program, whose end closes the viewer's stdin and so the window
		#[cfg(unix)]
		std::os::unix::process::CommandExt::process_group(&mut command, 0);
		let mut child = command.spawn().map_err(|failure| format!("paint: cannot start the viewer: {failure}"))?;
		if let Some(output) = child.stdout.take() {
			std::thread::spawn(move || {
				if follow_input(std::io::BufReader::new(output)) {
					end_the_run();
				}
				warp_runtime::system_values::tell_window_open(false); // its output ends as it closes
			});
		}
		warp_runtime::system_values::tell_window_open(true);
		*viewer = Some(child);
	}
	let input = viewer.as_mut().and_then(|child| child.stdin.as_mut()).expect("the viewer's stdin is piped");
	let asked = ASKED_SIZE.lock().ok().and_then(|mut asked| asked.take());
	let asked = asked.map(|(width, height)| resize_message(width, height)).unwrap_or_default();
	let written = input.write_all(&asked).and_then(|_| input.write_all(frame)).and_then(|_| input.flush());
	written.map_err(|failure| {
		*viewer = None;
		warp_runtime::system_values::tell_window_open(false);
		format!("paint: the viewer is gone ({failure})")
	})
}

/// A program that ended well leaves its window up until the user closes it (card exit-key): any other end closes it
pub fn wait_until_closed() {
	let viewer = VIEWER.lock().ok().and_then(|mut viewer| viewer.take());
	if let Some(mut child) = viewer {
		let _ = child.wait();
	}
}

/// `warp paint-window`: the window showing the frames arriving on stdin, until it is closed or its program ends
pub fn run(check: bool) -> Result<(), String> {
	let mut builder = EventLoop::<Message>::with_user_event();
	#[cfg(target_os = "macos")]
	if std::env::var_os(crate::graphics::paint::NO_WINDOW_VARIABLE).is_some() {
		use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
		// no Dock icon and no focus taken from the user's app (tests run beside them)
		builder.with_activation_policy(ActivationPolicy::Prohibited).with_activate_ignoring_other_apps(false);
	}
	let event_loop = builder.build().map_err(|failure| format!("paint-window: no event loop: {failure}"))?;
	let proxy = event_loop.create_proxy();
	std::thread::spawn(move || {
		let mut input = std::io::stdin().lock();
		while let Some(message) = read_message(&mut input) {
			if proxy.send_event(message).is_err() {
				return;
			}
		}
		// the program is gone (it ended, exit(), a failure, ctrl-c, a kill), so is its window; a check ends by itself
		if !check {
			std::process::exit(0);
		}
	});
	let mut viewer = Viewer { check, ..Viewer::default() };
	event_loop.run_app(&mut viewer).map_err(|failure| format!("paint-window: {failure}"))?;
	viewer.failure.map_or(Ok(()), Err)
}

#[derive(Default)]
struct Viewer {
	screen: Option<Screen>,
	/// What came before the window could take it
	waiting: Vec<Message>,
	failure: Option<String>,
	check: bool,
	/// The event loop runs: a window may open
	resumed: bool,
	/// The size in points the program asked for before the window opened
	asked: Option<(u32, u32)>,
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
	/// The canvas `use draw` keeps on the GPU, made by its first shapes
	canvas: Option<crate::graphics::gpu_canvas::Canvas>,
	/// The last shader frame to arrive, rendered at the next redraw: frames arriving faster than the window draws are
	/// skipped, not queued, so the picture keeps up with the program (and its music)
	newest_shader: Option<ShaderFrame>,
	frame_size: (u32, u32),
	pointer: (f64, f64),
	button_down: bool,
	/// the keys held, in the order they went down: the last is the key held (`held_key`)
	held: Vec<u32>,
	/// The size in points the program set (window.width = …): frames of another size no longer resize the window
	asked: Option<(u32, u32)>,
}

impl Viewer {
	/// The window, opened for the first frame (or shapes) at its size: none opens before one arrives
	fn open_for_waiting(&mut self, event_loop: &ActiveEventLoop) {
		let Some(first) = self.waiting.first().filter(|_| self.resumed && self.screen.is_none()) else {
			return;
		};
		let (width, height) = first.size();
		match Screen::open(event_loop, (width, height), self.asked, self.check) {
			Ok(screen) => self.screen = Some(screen),
			Err(failure) => {
				self.failure = Some(failure);
				return event_loop.exit();
			}
		}
		for message in std::mem::take(&mut self.waiting) {
			self.user_event(event_loop, message);
		}
	}
}

impl ApplicationHandler<Message> for Viewer {
	/// The user ended the window (closing it, Cmd-Q): its program ends too. A program that ends first ends the
	/// viewer through its stdin (run), a check ends by itself, and a failure is told as one
	fn exiting(&mut self, _: &ActiveEventLoop) {
		if !self.check && self.failure.is_none() {
			let _ = writeln!(std::io::stdout(), "{CLOSED_LINE}"); // a program gone meanwhile reads nothing
		}
	}

	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		self.resumed = true;
		self.open_for_waiting(event_loop);
	}

	fn user_event(&mut self, event_loop: &ActiveEventLoop, message: Message) {
		match (self.screen.as_mut(), message) {
			(Some(screen), Message::Frame(frame)) => screen.take(frame),
			(Some(screen), Message::Shapes(shapes)) => screen.draw_shapes(shapes),
			(Some(screen), Message::Shader(shader)) => {
				screen.newest_shader = Some(shader);
				screen.window.request_redraw();
			}
			(Some(screen), Message::Resize(width, height)) => screen.resize_to(width, height),
			(None, Message::Resize(width, height)) => self.asked = Some((width, height)),
			(None, message) => {
				self.waiting.push(message);
				self.open_for_waiting(event_loop);
			}
		}
	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
		let Some(screen) = self.screen.as_mut() else { return };
		match event {
			WindowEvent::CloseRequested => event_loop.exit(),
			WindowEvent::Resized(size) => {
				screen.resize(size.width, size.height);
				screen.tell_size();
			}
			WindowEvent::CursorMoved { position, .. } => {
				screen.pointer = screen.in_frame(position.x, position.y);
				screen.tell_input();
			}
			WindowEvent::MouseInput { state, .. } => {
				screen.button_down = state.is_pressed();
				screen.tell_input();
			}
			WindowEvent::KeyboardInput { event, .. } => {
				if let Some(code) = key_code(&event.logical_key) {
					screen.held.retain(|held| *held != code);
					if event.state.is_pressed() {
						screen.held.push(code);
					}
					screen.tell_input();
				}
			}
			// a key let go in another window never tells its release here
			WindowEvent::Focused(false) if !screen.held.is_empty() => {
				screen.held.clear();
				screen.tell_input();
			}
			WindowEvent::RedrawRequested => {
				if let Some(shader) = screen.newest_shader.take() {
					screen.render_shader(shader);
				}
				if let Some(center) = screen.draw(self.check) {
					println!("center {center}");
					event_loop.exit();
				}
			}
			_ => {}
		}
	}
}

/// The pointer's moves over the window reach it while another app is in front (card shader-brush): macOS sends mouse
/// moves only to the key window, and a viewer started from a terminal does not become it, so a brush following $mouse
/// stayed in its corner until a click. A tracking area active always sends them to winit's view (its mouseMoved:).
#[cfg(target_os = "macos")]
fn follow_pointer_unfocused(window: &Window) {
	use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
	let Ok(handle) = window.window_handle() else { return };
	let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
		return;
	};
	// SAFETY: ns_view is the window's live NSView
	track_pointer_always(unsafe { appkit.ns_view.cast().as_ref() });
}

/// A tracking area sending `view` the pointer's moves whether or not its window is the key window
#[cfg(target_os = "macos")]
pub fn track_pointer_always(view: &objc2::runtime::AnyObject) {
	use objc2::encode::{Encode, Encoding};
	use objc2::{class, msg_send, runtime::AnyObject};
	#[repr(C)]
	struct Pair(f64, f64);
	#[repr(C)]
	struct Rect(Pair, Pair);
	unsafe impl Encode for Pair {
		const ENCODING: Encoding = Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]);
	}
	unsafe impl Encode for Rect {
		const ENCODING: Encoding =
			Encoding::Struct("CGRect", &[Pair::ENCODING, Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING])]);
	}
	// SAFETY: NSTrackingArea's own initializer (the rect is ignored: IN_VISIBLE_RECT); the view keeps the area
	unsafe {
		let area: *mut AnyObject = msg_send![class!(NSTrackingArea), alloc];
		let area: *mut AnyObject = msg_send![area, initWithRect: Rect(Pair(0.0, 0.0), Pair(0.0, 0.0)), options: TRACKING_OPTIONS, owner: view, userInfo: std::ptr::null::<AnyObject>()];
		let _: () = msg_send![view, addTrackingArea: area];
	}
}

/// The window's inner size for a frame: a small one scaled up by a whole factor, a large one pixel for pixel
fn shown_size(width: u32, height: u32) -> winit::dpi::Size {
	let side = width.max(height).max(1);
	if side >= PIXEL_FOR_PIXEL_SIDE {
		return winit::dpi::PhysicalSize::new(width, height).into();
	}
	let scale = (SHOWN_SIDE / side).max(1);
	winit::dpi::LogicalSize::new(width * scale, height * scale).into()
}

impl Screen {
	/// The window for a first frame of `frame_size`, at the size in points the program `asked` for, else one fitting the frame
	fn open(
		event_loop: &ActiveEventLoop,
		frame_size: (u32, u32),
		asked: Option<(u32, u32)>,
		check: bool,
	) -> Result<Screen, String> {
		let size = asked.map_or(shown_size(frame_size.0, frame_size.1), |(width, height)| {
			winit::dpi::LogicalSize::new(width, height).into()
		});
		let attributes = Window::default_attributes().with_title(TITLE).with_inner_size(size);
		let window = Arc::new(
			event_loop.create_window(attributes).map_err(|failure| format!("paint-window: no window: {failure}"))?,
		);
		#[cfg(target_os = "macos")]
		follow_pointer_unfocused(&window);
		window.focus_window(); // its keys reach $key; macOS may still leave the terminal in front
		let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
		let surface = instance
			.create_surface(window.clone())
			.map_err(|failure| format!("paint-window: no surface: {failure}"))?;
		let options = wgpu::RequestAdapterOptions { compatible_surface: Some(&surface), ..Default::default() };
		let adapter = pollster::block_on(instance.request_adapter(&options))
			.map_err(|problem| format!("paint-window: no GPU adapter: {problem}"))?;
		let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
			.map_err(|problem| format!("paint-window: no GPU device: {problem}"))?;
		let size = window.inner_size();
		let mut config = surface
			.get_default_config(&adapter, size.width.max(1), size.height.max(1))
			.ok_or("paint-window: the surface does not fit the GPU")?;
		if check {
			config.usage |= wgpu::TextureUsages::COPY_SRC;
		}
		surface.configure(&device, &config);
		// the frame's bytes are sRGB like the surface's: sampled linear, written back as sRGB
		let texture_format =
			if config.format.is_srgb() { wgpu::TextureFormat::Rgba8UnormSrgb } else { wgpu::TextureFormat::Rgba8Unorm };
		let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: Some(TITLE),
			source: wgpu::ShaderSource::Wgsl(SHADER.into()),
		});
		let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some(TITLE),
			layout: None,
			vertex: wgpu::VertexState {
				module: &module,
				entry_point: Some("vertex"),
				compilation_options: Default::default(),
				buffers: &[],
			},
			fragment: Some(wgpu::FragmentState {
				module: &module,
				entry_point: Some("fragment"),
				compilation_options: Default::default(),
				targets: &[Some(config.format.into())],
			}),
			primitive: Default::default(),
			depth_stencil: None,
			multisample: Default::default(),
			multiview_mask: None,
			cache: None,
		});
		let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
		let screen = Screen {
			window,
			surface,
			config,
			device,
			queue,
			pipeline,
			sampler,
			texture_format,
			bindings: None,
			canvas: None,
			newest_shader: None,
			frame_size: (0, 0),
			pointer: (0.0, 0.0),
			button_down: false,
			held: vec![],
			asked,
		};
		screen.tell_size();
		Ok(screen)
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
			let layout = wgpu::TexelCopyBufferLayout {
				offset: 0,
				bytes_per_row: Some(frame.width * RGBA_BYTES as u32),
				rows_per_image: None,
			};
			self.queue.write_texture(texture.as_image_copy(), &frame.rgba, layout, size);
		}
		self.show_texture(&texture.create_view(&Default::default()), (frame.width, frame.height));
		self.window.request_redraw();
	}

	/// The shapes drawn over the canvas on the GPU, which the window then shows
	fn draw_shapes(&mut self, shapes: Shapes) {
		let canvas = self.canvas.get_or_insert_with(|| crate::graphics::gpu_canvas::Canvas::new(&self.device));
		let encoder = canvas.draw(&self.device, &self.queue, shapes.width, shapes.height, &shapes.shapes);
		self.queue.submit([encoder.finish()]);
		let view = canvas
			.texture()
			.create_view(&wgpu::TextureViewDescriptor { format: Some(self.texture_format), ..Default::default() });
		self.show_texture(&view, (shapes.width, shapes.height));
		self.window.request_redraw();
	}

	/// The shader rendered here into a texture the window then shows; a failure the shader shows only on this GPU
	/// goes to stderr, the program's terminal
	fn render_shader(&mut self, frame: ShaderFrame) {
		let ShaderFrame { width, height, shader, values } = frame;
		let format = Some(self.texture_format);
		match crate::graphics::gpu::render_texture(&self.device, &self.queue, &shader, width, height, &values, format) {
			Ok(image) => {
				let view = image.create_view(&wgpu::TextureViewDescriptor { format, ..Default::default() });
				self.show_texture(&view, (width, height));
			}
			Err(failure) => eprintln!("paint: {failure}"),
		}
	}

	/// The texture the next draw samples, `size` pixels (the caller asks for that draw)
	fn show_texture(&mut self, view: &wgpu::TextureView, size: (u32, u32)) {
		self.bindings = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some(TITLE),
			layout: &self.pipeline.get_bind_group_layout(0),
			entries: &[
				wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
				wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
			],
		}));
		if self.frame_size != size && self.frame_size != (0, 0) && self.asked.is_none() {
			let _ = self.window.request_inner_size(shown_size(size.0, size.1));
		}
		if self.frame_size == (0, 0) {
			// before the pointer moves over the window it is in the middle of the first frame, where a brush following
			// $mouse shows (user 2026-10-12, card shader-brush), not under the window's rounded corner
			self.pointer = (f64::from(size.0) / 2.0, f64::from(size.1) / 2.0);
			self.tell_input();
		}
		self.frame_size = size;
	}

	/// window.width = …: the window takes that size in points and keeps it
	fn resize_to(&mut self, width: u32, height: u32) {
		self.asked = Some((width, height));
		let _ = self.window.request_inner_size(winit::dpi::LogicalSize::new(width, height));
	}

	/// The window's inner size in points as the program's side takes it (follow_input)
	fn tell_size(&self) {
		let size = self.window.inner_size().to_logical::<f64>(self.window.scale_factor());
		let mut output = std::io::stdout().lock();
		let _ =
			writeln!(output, "{SIZE_LINE} {} {}", size.width.round(), size.height.round()).and_then(|_| output.flush());
	}

	/// The key held now: the last to go down of those held, 0 for none
	fn key(&self) -> u32 {
		self.held.last().copied().unwrap_or(0)
	}

	/// The input as the program's side takes it (follow_input)
	fn tell_input(&self) {
		// the window outlives the program by design: once that exits, its end of the pipe is closed and nobody listens
		let mut output = std::io::stdout().lock();
		let held: String = self.held.iter().map(|code| format!(" {code}")).collect();
		let _ = writeln!(
			output,
			"{INPUT_LINE} {} {} {} {}{held}",
			self.pointer.0,
			self.pointer.1,
			u8::from(self.button_down),
			self.key()
		)
		.and_then(|_| output.flush());
	}

	/// A place in the window's physical pixels as one in the frame's pixels
	fn in_frame(&self, x: f64, y: f64) -> (f64, f64) {
		let (frame_width, frame_height) = self.frame_size;
		(
			x * f64::from(frame_width) / f64::from(self.config.width.max(1)),
			y * f64::from(frame_height) / f64::from(self.config.height.max(1)),
		)
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
			let attachment = wgpu::RenderPassColorAttachment {
				view: &target,
				depth_slice: None,
				resolve_target: None,
				ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
			};
			let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				color_attachments: &[Some(attachment)],
				..Default::default()
			});
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
		let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
			label: Some(TITLE),
			size: COPY_ROW_BYTES.into(),
			usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});
		let mut center = texture.as_image_copy();
		center.origin = wgpu::Origin3d { x: self.config.width / 2, y: self.config.height / 2, z: 0 };
		let layout =
			wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(COPY_ROW_BYTES), rows_per_image: None };
		encoder.copy_texture_to_buffer(
			center,
			wgpu::TexelCopyBufferInfo { buffer: &buffer, layout },
			wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
		);
		buffer
	}

	/// `r g b` of the copied pixel, whatever the order of the surface's format
	fn center_of(&self, buffer: &wgpu::Buffer) -> String {
		buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
		if let Err(problem) = self.device.poll(wgpu::PollType::wait_indefinitely()) {
			return format!("unread: {problem}");
		}
		let Ok(bytes) = buffer.slice(..).get_mapped_range() else {
			return "unread".into();
		};
		let blue_first =
			matches!(self.config.format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
		let [red, green, blue] =
			if blue_first { [bytes[2], bytes[1], bytes[0]] } else { [bytes[0], bytes[1], bytes[2]] };
		format!("{red} {green} {blue}")
	}
}

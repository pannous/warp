//! paint(pixels, width, height) natively (card native-paint): the pixels as a PNG in the system's temporary
//! folder (warp-paint/paint.png, then paint-2.png … within one run; the next run overwrites them, so no project folder
//! collects images), named on stderr; run by `warp` itself a window shows them instead (src/graphics/paint_window.rs), also
//! from an editor's build or a pipe; never in tests (WARP_NO_WINDOW, CI, or warp used as a library). The playground draws the same pixels on a canvas (web/playground/playground.js showPaintings).

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub use super::pixels::{COLOR_FROM, INK, PAPER};
const FILE_STEM: &str = "paint";
const FOLDER: &str = "warp-paint";
/// The runtime warning of a warp function painted on the CPU for want of a GPU (paints_shaders), its reason after it
const PAINTS_ON_CPU: &str = "paint: a function paints on the CPU";
/// Set when tests or a headless machine must not see windows (tests/common warp_command, tests/queue.sh)
pub const NO_WINDOW_VARIABLE: &str = "WARP_NO_WINDOW";
const CI_VARIABLE: &str = "CI";
/// `WARP_DRAW=gpu` or `cpu` picks how `use draw` draws (src/graphics/gpu_canvas.rs); without it, on the GPU when a window shows
pub const DRAW_VARIABLE: &str = "WARP_DRAW";
const DRAW_CHOICES: [&str; 2] = ["gpu", "cpu"];
/// Whether paint shows a window: only the warp binary turns it on (allow_windows); a library caller writes PNGs
static WINDOWS: AtomicBool = AtomicBool::new(false);
/// The paint calls of this run so far
static PAINTED: AtomicUsize = AtomicUsize::new(0);
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
const GRAYSCALE_8_BIT: [u8; 5] = [8, 0, 0, 0, 0]; // bit depth, color type gray, deflate, filter method, no interlace
const RGB_8_BIT: [u8; 5] = [8, 2, 0, 0, 0]; // color type truecolor

/// What a pixel value shows: paper for 0, its color for a value with an alpha byte, else ink
pub fn shade(value: u64) -> [u8; 3] {
	match value {
		0 => [PAPER; 3],
		color if color >= COLOR_FROM => [(color >> 16) as u8, (color >> 8) as u8, color as u8],
		_ => [INK; 3],
	}
}

/// Whether WARP_NO_WINDOW or CI forbids any window: tests, agents and headless machines
pub fn headless() -> bool {
	[NO_WINDOW_VARIABLE, CI_VARIABLE].iter().any(|variable| std::env::var_os(variable).is_some())
}

/// The warp binary's paint calls show windows, unless WARP_NO_WINDOW or CI is set (main.rs)
pub fn allow_windows() {
	let headless = headless();
	WINDOWS.store(!headless, Ordering::Relaxed);
	let chosen = std::env::var(DRAW_VARIABLE).ok().filter(|choice| !choice.is_empty());
	if let Some(unknown) = chosen.as_ref().filter(|choice| !DRAW_CHOICES.contains(&choice.as_str())) {
		eprintln!(
			"warning: {DRAW_VARIABLE}={unknown} is neither {}: use draw draws as without it",
			DRAW_CHOICES.join(" nor ")
		);
	}
	let on_gpu = chosen
		.as_deref()
		.filter(|choice| DRAW_CHOICES.contains(choice))
		.map_or(!headless, |choice| choice == DRAW_CHOICES[0]);
	warp_runtime::system_values::tell_gpu_drawing(on_gpu);
	if !headless {
		warp_runtime::system_values::tell_window_opener(open_blank_window);
		warp_runtime::system_values::tell_window_resizer(crate::graphics::paint_window::resize);
	}
}

/// A blank window of the native view's size, for a program asking window_open before its first paint (card window-loop)
fn open_blank_window() {
	let (width, height) = warp_runtime::system_values::native_view();
	let (width, height) = (width as usize, height as usize);
	let paper = vec![0; width * height];
	if let Err(failure) =
		crate::graphics::paint_window::show(&crate::graphics::paint_window::frame_bytes(&paper, width, height))
	{
		eprintln!("window_open: {failure}");
	}
}

/// Whether the warp binary may reach the user's screen and speakers: paint's windows, sound's player (src/sound/mod.rs)
pub fn shows_windows() -> bool {
	WINDOWS.load(Ordering::Relaxed)
}

/// Show the image in a window (paint_window.rs) when windows are allowed, else (or without a window) write it, say where
pub fn paint(pixels: &[u64], width: usize, height: usize) -> Result<Option<PathBuf>, String> {
	if width == 0 || height == 0 {
		return Ok(None); // nothing to show, as the playground's canvas (user 2026-10-10: defaults, no errors)
	}
	if pixels.len() < width * height {
		return Err(format!("paint: {width}×{height} needs {} pixels, got {}", width * height, pixels.len()));
	}
	shown_or_written(
		|| crate::graphics::paint_window::frame_bytes(pixels, width, height),
		|| png_file(pixels, width, height),
	)
}

/// paint of a rendered shader (gpu::render_rgba): its RGBA bytes go to the window as they are
pub fn paint_rgba(rgba: &[u8], width: usize, height: usize) -> Result<Option<PathBuf>, String> {
	shown_or_written(
		|| crate::graphics::paint_window::rgba_frame(rgba, width, height),
		|| rgba_png_file(rgba, width, height),
	)
}

fn rgba_png_file(rgba: &[u8], width: usize, height: usize) -> Result<PathBuf, String> {
	png_file(&rgba_pixels(rgba), width, height)
}

/// RGBA bytes as pixel values, each an opaque color
fn rgba_pixels(rgba: &[u8]) -> Vec<u64> {
	crate::graphics::gpu::opaque_colors(rgba).map(u64::from).collect()
}

/// Whether a shader can paint (system value GPU_PAINTS): in a window the window's process renders it, else this process's
/// GPU does; without one a warp function paints on the CPU, said once
pub fn paints_shaders() -> bool {
	if shows_windows() {
		return true;
	}
	match crate::graphics::gpu::available() {
		Ok(()) => true,
		Err(problem) => {
			crate::errors::diagnostic::report_runtime_warning_once(PAINTS_ON_CPU, &problem);
			false
		}
	}
}

/// paint of a shader: in a window the window's process renders it (card paint-keeps), only the shader and its values
/// cross; else its pixels as a PNG. A shader that does not compile fails here either way
pub fn paint_shader(
	shader: &str,
	width: u32,
	height: u32,
	values: &[crate::graphics::gpu::ShaderValue],
) -> Result<Option<PathBuf>, String> {
	if shows_windows() {
		let values = crate::graphics::gpu::checked_values(shader, width, height, values)?;
		if shown(&crate::graphics::paint_window::shader_message(width, height, shader, &values)) {
			return Ok(None);
		}
	}
	let rgba = crate::graphics::gpu::render_rgba(shader, width, height, values)?;
	rgba_png_file(&rgba, width as usize, height as usize).map(Some)
}

/// The frame in a window when windows are allowed, else (or without a window) the PNG written
fn shown_or_written(
	frame: impl FnOnce() -> Vec<u8>,
	write: impl FnOnce() -> Result<PathBuf, String>,
) -> Result<Option<PathBuf>, String> {
	if shows_windows() && shown(&frame()) {
		return Ok(None);
	}
	write().map(Some)
}

/// Whether the window shows the message; its failure said, as the picture goes to a PNG instead
fn shown(message: &[u8]) -> bool {
	crate::graphics::paint_window::show(message).map_err(|failure| eprintln!("{failure}, so it goes to a PNG")).is_ok()
}

/// The image as a PNG in the temporary folder
fn png_file(pixels: &[u64], width: usize, height: usize) -> Result<PathBuf, String> {
	let call = PAINTED.fetch_add(1, Ordering::Relaxed) + 1;
	let png = png(pixels, width, height);
	let path = crate::extensions::utils::numbered_temporary_file(FOLDER, FILE_STEM, call, "png", &png, "paint")?;
	eprintln!("painted {width}×{height}: {}", path.display());
	Ok(path)
}

/// `save canvas as "painting.png"` (card save-painting, lib/draw.warp): the pixels as a PNG at that path
pub fn save_png(pixels: &[u64], width: usize, height: usize, path: &str) -> Result<(), String> {
	if pixels.len() < width * height {
		return Err(format!("save: a {width}×{height} picture needs {} pixels, got {}", width * height, pixels.len()));
	}
	std::fs::write(path, png(pixels, width, height)).map_err(|failure| format!("save: cannot write {path}: {failure}"))
}

/// A picture the GPU drew (gpu_canvas::snapshot) as a PNG at that path
pub fn save_rgba_png(rgba: &[u8], width: usize, height: usize, path: &str) -> Result<(), String> {
	save_png(&rgba_pixels(rgba), width, height, path)
}

/// A grayscale PNG of ink and paper, a truecolor one when a pixel has a color
fn png(pixels: &[u64], width: usize, height: usize) -> Vec<u8> {
	let colored = pixels.iter().any(|&value| value >= COLOR_FROM);
	let mut rows = Vec::with_capacity((width * if colored { 3 } else { 1 } + 1) * height);
	for row in pixels.chunks(width).take(height) {
		rows.push(0); // filter: none
		for &value in row {
			match colored {
				true => rows.extend(shade(value)),
				false => rows.push(shade(value)[0]),
			}
		}
	}
	let mut deflated = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast()); // fast: an animation writes a frame each paint
	deflated.write_all(&rows).expect("writing to memory");
	let header = [
		(width as u32).to_be_bytes().as_slice(),
		&(height as u32).to_be_bytes(),
		if colored { &RGB_8_BIT } else { &GRAYSCALE_8_BIT },
	]
	.concat();
	let mut file = PNG_SIGNATURE.to_vec();
	for (kind, data) in [(b"IHDR", header), (b"IDAT", deflated.finish().expect("writing to memory")), (b"IEND", vec![])]
	{
		file.extend((data.len() as u32).to_be_bytes());
		let checked = [kind.as_slice(), &data].concat();
		file.extend(&checked);
		file.extend(crc32fast::hash(&checked).to_be_bytes());
	}
	file
}

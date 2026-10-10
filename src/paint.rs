//! paint(pixels, width, height) natively (card native-paint): the pixels as a PNG in the system's temporary
//! folder (warp-paint/paint.png, then paint-2.png … within one run; the next run overwrites them, so no project folder
//! collects images), named on stderr; run by `warp` itself a window shows them instead (src/paint_window.rs), also
//! from an editor's build or a pipe; never in tests (WARP_NO_WINDOW, CI, or warp used as a library). The playground draws the same pixels on a canvas (web/playground/playground.js showPaintings).

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// The gray levels of a nonzero pixel and of a zero one, as the playground draws them
pub const INK: u8 = 29;
pub const PAPER: u8 = 250;
/// A pixel value from here on carries an alpha byte, 0xAARRGGBB: it is that color (lib/draw.warp), any smaller nonzero
/// value is ink (playground.js COLOR_FROM)
pub const COLOR_FROM: u64 = 1 << 24;
const FILE_STEM: &str = "paint";
const FOLDER: &str = "warp-paint";
/// Set when tests or a headless machine must not see windows (tests/common warp_command, tests/queue.sh)
pub const NO_WINDOW_VARIABLE: &str = "WARP_NO_WINDOW";
const CI_VARIABLE: &str = "CI";
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

/// The warp binary's paint calls show windows, unless WARP_NO_WINDOW or CI is set (main.rs)
pub fn allow_windows() {
	let headless = [NO_WINDOW_VARIABLE, CI_VARIABLE].iter().any(|variable| std::env::var_os(variable).is_some());
	WINDOWS.store(!headless, Ordering::Relaxed);
}

/// Whether the warp binary may reach the user's screen and speakers: paint's windows, sound's player (src/sound.rs)
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
	if WINDOWS.load(Ordering::Relaxed) {
		match crate::paint_window::show(pixels, width, height) {
			Ok(()) => return Ok(None),
			Err(failure) => eprintln!("{failure}, so it goes to a PNG"),
		}
	}
	png_file(pixels, width, height).map(Some)
}

/// The image as a PNG in the temporary folder
fn png_file(pixels: &[u64], width: usize, height: usize) -> Result<PathBuf, String> {
	let folder = std::env::temp_dir().join(FOLDER);
	std::fs::create_dir_all(&folder).map_err(|failure| format!("paint: cannot create {}: {failure}", folder.display()))?;
	let path = folder.join(file_name(PAINTED.fetch_add(1, Ordering::Relaxed) + 1));
	std::fs::write(&path, png(pixels, width, height)).map_err(|failure| format!("paint: cannot write {}: {failure}", path.display()))?;
	eprintln!("painted {width}×{height}: {}", path.display());
	Ok(path)
}

/// paint.png for the first call of a run, paint-2.png for the second …
fn file_name(call: usize) -> String {
	if call == 1 { format!("{FILE_STEM}.png") } else { format!("{FILE_STEM}-{call}.png") }
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
	let mut deflated = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
	deflated.write_all(&rows).expect("writing to memory");
	let header = [(width as u32).to_be_bytes().as_slice(), &(height as u32).to_be_bytes(), if colored { &RGB_8_BIT } else { &GRAYSCALE_8_BIT }].concat();
	let mut file = PNG_SIGNATURE.to_vec();
	for (kind, data) in [(b"IHDR", header), (b"IDAT", deflated.finish().expect("writing to memory")), (b"IEND", vec![])] {
		file.extend((data.len() as u32).to_be_bytes());
		let checked = [kind.as_slice(), &data].concat();
		file.extend(&checked);
		file.extend(crc32fast::hash(&checked).to_be_bytes());
	}
	file
}

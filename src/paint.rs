//! paint(pixels, width, height) natively (card native-paint): the pixels as a grayscale PNG in the system's temporary
//! folder (warp-paint/paint.png, then paint-2.png … within one run; the next run overwrites them, so no project folder
//! collects images), named on stderr and opened in the system viewer when stderr is a terminal (never in tests or pipes). The playground draws the same pixels on a canvas (web/playground/playground.js showPaintings).

use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The gray levels of a nonzero pixel and of a zero one, as the playground draws them
pub const INK: u8 = 29;
pub const PAPER: u8 = 250;
const FILE_STEM: &str = "paint";
const FOLDER: &str = "warp-paint";
/// The paint calls of this run so far
static PAINTED: AtomicUsize = AtomicUsize::new(0);
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
const GRAYSCALE_8_BIT: [u8; 5] = [8, 0, 0, 0, 0]; // bit depth, color type gray, deflate, filter method, no interlace
#[cfg(target_os = "macos")]
const VIEWER: &str = "open";
#[cfg(not(target_os = "macos"))]
const VIEWER: &str = "xdg-open";

/// Write the image, say where, show it on a terminal
pub fn paint(ink: &[bool], width: usize, height: usize) -> Result<PathBuf, String> {
	if ink.len() < width * height {
		return Err(format!("paint: {width}×{height} needs {} pixels, got {}", width * height, ink.len()));
	}
	let folder = std::env::temp_dir().join(FOLDER);
	std::fs::create_dir_all(&folder).map_err(|failure| format!("paint: cannot create {}: {failure}", folder.display()))?;
	let path = folder.join(file_name(PAINTED.fetch_add(1, Ordering::Relaxed) + 1));
	std::fs::write(&path, png(ink, width, height)).map_err(|failure| format!("paint: cannot write {}: {failure}", path.display()))?;
	eprintln!("painted {width}×{height}: {}", path.display());
	if std::io::stderr().is_terminal() {
		let _ = std::process::Command::new(VIEWER).arg(&path).spawn();
	}
	Ok(path)
}

/// paint.png for the first call of a run, paint-2.png for the second …
fn file_name(call: usize) -> String {
	if call == 1 { format!("{FILE_STEM}.png") } else { format!("{FILE_STEM}-{call}.png") }
}

fn png(ink: &[bool], width: usize, height: usize) -> Vec<u8> {
	let mut rows = Vec::with_capacity((width + 1) * height);
	for row in ink.chunks(width).take(height) {
		rows.push(0); // filter: none
		rows.extend(row.iter().map(|&dark| if dark { INK } else { PAPER }));
	}
	let mut deflated = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
	deflated.write_all(&rows).expect("writing to memory");
	let header = [(width as u32).to_be_bytes().as_slice(), &(height as u32).to_be_bytes(), &GRAYSCALE_8_BIT].concat();
	let mut file = PNG_SIGNATURE.to_vec();
	for (kind, data) in [(b"IHDR", header), (b"IDAT", deflated.finish().expect("writing to memory")), (b"IEND", vec![])] {
		file.extend((data.len() as u32).to_be_bytes());
		let checked = [kind.as_slice(), &data].concat();
		file.extend(&checked);
		file.extend(crc32fast::hash(&checked).to_be_bytes());
	}
	file
}

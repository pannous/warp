// paint(pixels, width, height) draws on the canvas of the browser playground (issue #15); natively it writes a PNG in
// the temporary folder (warp-paint/paint.png) and names it on stderr (card native-paint)
#![cfg(feature = "native")]
use std::io::Read;

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

#[test]
fn paint_writes_a_png_natively() {
	let run = crate::common::warp_command()
		.args(["--no-ask", "eval", "paint([0, 1, 1, 0, 0, 1], 3, 2); 7"])
		.output()
		.unwrap();
	let errors = String::from_utf8_lossy(&run.stderr);
	assert!(String::from_utf8_lossy(&run.stdout).contains('7'), "{errors}");
	let path = errors
		.lines()
		.find_map(|line| line.strip_prefix("painted 3×2: "))
		.unwrap_or_else(|| panic!("no painted line: {errors}"));
	assert!(path.ends_with("warp-paint/paint.png"), "{path}");
	let png = std::fs::read(path).expect("paint.png written");
	assert!(png.starts_with(PNG_SIGNATURE));
	assert_eq!(
		(u32::from_be_bytes(png[16..20].try_into().unwrap()), u32::from_be_bytes(png[20..24].try_into().unwrap())),
		(3, 2)
	);
	let data_start = png.windows(4).position(|window| window == b"IDAT").unwrap() + 4;
	let data_length = u32::from_be_bytes(png[data_start - 8..data_start - 4].try_into().unwrap()) as usize;
	let mut rows = vec![];
	flate2::read::ZlibDecoder::new(&png[data_start..data_start + data_length]).read_to_end(&mut rows).unwrap();
	// each row: filter byte 0, then a gray level per pixel, ink where the value is nonzero
	assert_eq!(rows, [0, 250, 29, 29, 0, 250, 250, 29]);
}

#[test]
fn paint_writes_colors_as_a_truecolor_png() {
	// a folder of its own: the other paint test writes warp-paint/paint.png of the shared temporary folder at the same time
	let folder = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("paint-colors");
	std::fs::create_dir_all(&folder).unwrap();
	let run = crate::common::warp_command()
		.env("TMPDIR", &folder)
		.args(["--no-ask", "eval", "paint([0xFFFF8000, 0, 1], 3, 1); 7"])
		.output()
		.unwrap();
	let errors = String::from_utf8_lossy(&run.stderr);
	let path = errors
		.lines()
		.find_map(|line| line.strip_prefix("painted 3×1: "))
		.unwrap_or_else(|| panic!("no painted line: {errors}"));
	let png = std::fs::read(path).expect("png written");
	assert_eq!(png[25], 2, "color type truecolor");
	let data_start = png.windows(4).position(|window| window == b"IDAT").unwrap() + 4;
	let data_length = u32::from_be_bytes(png[data_start - 8..data_start - 4].try_into().unwrap()) as usize;
	let mut rows = vec![];
	flate2::read::ZlibDecoder::new(&png[data_start..data_start + data_length]).read_to_end(&mut rows).unwrap();
	// the color, paper for 0, ink for any other value without an alpha byte
	assert_eq!(rows, [0, 255, 128, 0, 250, 250, 250, 29, 29, 29]);
}

// card g_gGsg: in a terminal paint sends each frame to the window of `warp paint-window` (src/graphics/paint_window.rs) over its
// stdin, as [width u32][height u32][RGBA bytes] shaded as the PNG is; the viewer reads back the same frames (the window
// itself: probes/paint_window.sh, not here, where no window may open)
#[test]
fn paint_frames_for_the_window_read_back_as_sent() {
	use warp::graphics::paint_window::{frame_bytes, read_frame, Frame};
	let paper = warp::graphics::paint::PAPER;
	let ink = warp::graphics::paint::INK;
	let mut stream = [frame_bytes(&[0, 1, 0xFF12_C863], 3, 1), frame_bytes(&[1], 1, 1)].concat();
	let mut reader = std::io::Cursor::new(&mut stream);
	assert_eq!(
		read_frame(&mut reader),
		Some(Frame {
			width: 3,
			height: 1,
			rgba: vec![paper, paper, paper, 255, ink, ink, ink, 255, 0x12, 0xC8, 0x63, 255]
		})
	);
	assert_eq!(read_frame(&mut reader), Some(Frame { width: 1, height: 1, rgba: vec![ink, ink, ink, 255] }));
	assert_eq!(read_frame(&mut reader), None);
}

// card shader-brush: macOS sends mouse moves only to the key window, and the paint window started from a terminal is
// not it, so samples/shader_brush.warp's glow never followed the hovering pointer; the viewer's view gets a tracking
// area active always (src/graphics/paint_window.rs track_pointer_always). Checked on a view of no window: none opens here
#[test]
#[cfg(all(feature = "native", target_os = "macos"))]
fn the_paint_window_hears_the_pointer_unfocused() {
	use objc2::{class, msg_send, runtime::AnyObject};
	// SAFETY: a fresh NSView and AppKit's own getters on it
	unsafe {
		let view: *mut AnyObject = msg_send![class!(NSView), new];
		warp::graphics::paint_window::track_pointer_always(&*view);
		let areas: *mut AnyObject = msg_send![&*view, trackingAreas];
		let count: usize = msg_send![&*areas, count];
		assert_eq!(count, 1);
		let area: *mut AnyObject = msg_send![&*areas, firstObject];
		let options: usize = msg_send![&*area, options];
		assert_ne!(options & warp::graphics::paint_window::TRACKING_ACTIVE_ALWAYS, 0, "options {options:#x}");
	}
}

// paint(pixels, width, height) draws on the canvas of the browser playground (issue #15); natively it writes a PNG in
// the temporary folder (warp-paint/paint.png) and names it on stderr (card native-paint)
#![cfg(feature = "native")]
use std::io::Read;

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

#[test]
fn paint_writes_a_png_natively() {
	let run = crate::common::warp_command().args(["--no-ask", "eval", "paint([0, 1, 1, 0, 0, 1], 3, 2); 7"]).output().unwrap();
	let errors = String::from_utf8_lossy(&run.stderr);
	assert!(String::from_utf8_lossy(&run.stdout).contains('7'), "{errors}");
	let path = errors.lines().find_map(|line| line.strip_prefix("painted 3×2: ")).unwrap_or_else(|| panic!("no painted line: {errors}"));
	assert!(path.ends_with("warp-paint/paint.png"), "{path}");
	let png = std::fs::read(path).expect("paint.png written");
	assert!(png.starts_with(PNG_SIGNATURE));
	assert_eq!((u32::from_be_bytes(png[16..20].try_into().unwrap()), u32::from_be_bytes(png[20..24].try_into().unwrap())), (3, 2));
	let data_start = png.windows(4).position(|window| window == b"IDAT").unwrap() + 4;
	let data_length = u32::from_be_bytes(png[data_start - 8..data_start - 4].try_into().unwrap()) as usize;
	let mut rows = vec![];
	flate2::read::ZlibDecoder::new(&png[data_start..data_start + data_length]).read_to_end(&mut rows).unwrap();
	// each row: filter byte 0, then a gray level per pixel, ink where the value is nonzero
	assert_eq!(rows, [0, 250, 29, 29, 0, 250, 250, 29]);
}

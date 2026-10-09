// card g_odW4: paint reads a list of Ints in one call of the module's list_to_ints (src/wasm_emitter/int_lists.rs);
// any other pixel (a float, a bool, a negative) is read node by node as before, and a short list still fails loudly
#![cfg(feature = "native")]
use std::io::Read;

/// The gray levels of the PNG `code` paints into a folder of its own, row by row after each filter byte
pub(crate) fn painted_rows(code: &str, folder: &str) -> Vec<u8> {
	let folder = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(folder);
	std::fs::create_dir_all(&folder).unwrap();
	let run = crate::common::warp_command().env("TMPDIR", &folder).args(["--no-ask", "eval", code]).output().unwrap();
	let errors = String::from_utf8_lossy(&run.stderr);
	let path = errors.lines().find_map(|line| line.split_once(": ").filter(|(said, _)| said.starts_with("painted ")).map(|(_, path)| path.to_string()));
	let png = std::fs::read(path.unwrap_or_else(|| panic!("no painted line: {errors}"))).unwrap();
	let data_start = png.windows(4).position(|window| window == b"IDAT").unwrap() + 4;
	let data_length = u32::from_be_bytes(png[data_start - 8..data_start - 4].try_into().unwrap()) as usize;
	let mut rows = vec![];
	flate2::read::ZlibDecoder::new(&png[data_start..data_start + data_length]).read_to_end(&mut rows).unwrap();
	rows
}

#[test]
fn paint_reads_ints_from_a_built_list() {
	let rows = painted_rows("pixels = []; for i in 0..6 { pixels.push(i % 2) }; paint(pixels, 3, 2)", "paint-built");
	assert_eq!(rows, [0, 250, 29, 250, 0, 29, 250, 29]);
}

#[test]
fn paint_reads_other_pixels_node_by_node() {
	assert_eq!(painted_rows("paint([-1, 0, 2.5, true, false], 5, 1)", "paint-mixed"), [0, 29, 250, 29, 29, 250]);
}

#[test]
fn paint_takes_the_first_pixels_of_a_longer_list() {
	assert_eq!(painted_rows("paint([1, 0, 1, 1, 1], 2, 1)", "paint-longer"), [0, 29, 250]);
}

#[test]
fn paint_of_too_few_ints_fails() {
	let run = crate::common::warp_command().args(["--no-ask", "eval", "paint([1, 0], 3, 1)"]).output().unwrap();
	let said = format!("{}{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));
	assert!(said.contains("paint: 3×1 needs 3 pixels, got 2"), "{said}");
}

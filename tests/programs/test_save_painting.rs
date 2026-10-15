// card save-painting: `save "painting.png"`, `save canvas as: "painting.png"` and `save(canvas, "painting.png")` of
// use draw write the canvas as drawn so far as a PNG (decision save-sound: save is one word, overloaded by what it
// saves), drawn here on the CPU and, with WARP_DRAW=gpu, on the GPU, whose canvas the host draws again to save it
#![cfg(feature = "native")]

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
/// IHDR's width and height (4 × 3) and its truecolor color type, after the signature and the chunk's length and name
const SIZE_AT: std::ops::Range<usize> = 16..24;
const COLOR_TYPE_AT: usize = 25;
const TRUECOLOR: u8 = 2;

fn saved_path(name: &str) -> std::path::PathBuf {
	let folder = std::env::temp_dir().join(format!("warp-save-painting-{}", std::process::id()));
	std::fs::create_dir_all(&folder).unwrap();
	folder.join(format!("{name}.png"))
}

fn program_saving(statement: &str) -> String {
	format!("use draw\ncanvas(4, 3)\nclear(red)\n{statement}\n")
}

fn assert_red_png(path: &std::path::Path) {
	let png = std::fs::read(path).unwrap_or_else(|_| panic!("nothing saved at {}", path.display()));
	assert!(png.starts_with(PNG_SIGNATURE), "{}", path.display());
	assert_eq!(&png[SIZE_AT], &[0, 0, 0, 4, 0, 0, 0, 3]);
	assert_eq!(png[COLOR_TYPE_AT], TRUECOLOR);
}

#[test]
fn save_writes_the_canvas_as_a_png() {
	for (name, form) in
		[("plain", "save \"PATH\""), ("as", "save canvas as: \"PATH\""), ("call", "save(canvas, \"PATH\")")]
	{
		let path = saved_path(name);
		warp::wasm_emitter::eval(&program_saving(&form.replace("PATH", &path.display().to_string())));
		assert_red_png(&path);
	}
}

#[test]
fn save_writes_the_gpu_canvas_as_a_png() {
	if warp::graphics::gpu::available().is_err() {
		return crate::common::announce_skip("a GPU", module_path!());
	}
	let path = saved_path("gpu");
	let program = program_saving(&format!("save canvas as: \"{}\"", path.display()));
	let run =
		crate::common::warp_command().env("WARP_DRAW", "gpu").args(["--no-ask", "eval", &program]).output().unwrap();
	assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
	assert_red_png(&path);
}

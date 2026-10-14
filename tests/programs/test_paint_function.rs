// card gpu-functions step 4: paint(f, width, height) of a warp function f(x, y) paints f's value at each pixel's column
// and row. Where WGSL can compute f it becomes a fragment shader (f32: user 2026-10-13, precision doesn't matter on the
// GPU; colors stay exact as vec4f), the program's numbers it reads its values; otherwise, under @cpu or WARP_GPU=off
// the CPU computes f pixel by pixel. A number is ink unless 0, as in paint(pixels, …)
#![cfg(feature = "native")]
use super::test_paint_lists::{painted_rows, painted_rows_by};

const MANDELBROT: &str = "width = 48
height = 32
max_iter = 30
def escape_time(cx, cy) {
	x = 0.0 as float
	y = 0.0 as float
	iter = 0
	while x*x + y*y <= 4 and iter < max_iter {
		xtemp = x*x - y*y + cx
		y = 2*x*y + cy
		x = xtemp
		iter = iter + 1
	}
	return iter
}
def ink(col, row) {
	steps = escape_time(-2.2 + col * 3.2 / width, -1.1 + row * 2.2 / height)
	steps == max_iter or steps % 2 == 1
}
";
const COLORED: &str = "use draw
width = 24
height = 16
stripes(x, y) := (x + y) % 8 < 4 ? rgb(255, 128, 0) : hsv(240 * y / height, 1, 1)
";
/// f32 against f64 near the set's edge: an escape a step earlier or later flips a pixel
const MOST_FLIPPED_SHARE: f64 = 0.03;
/// A channel the GPU rounds from f32 may differ by one from the CPU's
const CHANNEL_TOLERANCE: u8 = 1;

fn has_gpu() -> bool {
	warp::graphics::gpu::with_device(|_, _| ()).is_ok()
}

fn lowered(program: &str) -> String {
	warp::pipeline::lower(program).expect("a program").serialize()
}

/// The pixels as RGB triples: a grayscale PNG's levels repeated
fn rgb_pixels(rows: &[u8], width: usize) -> Vec<[u8; 3]> {
	let colored = rows.len().is_multiple_of(3 * width + 1) && !rows.len().is_multiple_of(width + 1);
	let row_bytes = if colored { 3 * width + 1 } else { width + 1 };
	rows.chunks(row_bytes)
		.flat_map(|row| match colored {
			true => row[1..].chunks(3).map(|rgb| [rgb[0], rgb[1], rgb[2]]).collect::<Vec<_>>(),
			false => row[1..].iter().map(|&level| [level; 3]).collect(),
		})
		.collect()
}

/// `paint(function, width, height)` after `program`, on the GPU and on the CPU (WARP_GPU=off)
fn painted_both_ways(program: &str, function: &str, width: usize, folder: &str) -> (Vec<[u8; 3]>, Vec<[u8; 3]>) {
	let code = format!("{program}paint({function}, width, height)");
	let mut on_cpu = crate::common::warp_command();
	on_cpu.env("WARP_GPU", "off");
	(
		rgb_pixels(&painted_rows(&code, &format!("{folder}-gpu")), width),
		rgb_pixels(&painted_rows_by(on_cpu, &code, &format!("{folder}-cpu")), width),
	)
}

#[test]
fn a_painted_function_becomes_a_fragment_shader() {
	let mandelbrot = lowered(&format!("{MANDELBROT}paint(ink, width, height)"));
	assert!(mandelbrot.contains("@fragment") && mandelbrot.contains("fn warp_escape_time("), "{mandelbrot}");
	let colored = lowered(&format!("{COLORED}paint(stripes, width, height)"));
	assert!(colored.contains("fn warp_hsv_color("), "{colored}");
}

#[test]
fn a_painted_function_paints_on_the_gpu_what_the_cpu_paints() {
	if !has_gpu() {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	let (gpu, cpu) = painted_both_ways(MANDELBROT, "ink", 48, "paint-mandelbrot");
	assert_eq!(gpu.len(), 48 * 32);
	let flipped = gpu.iter().zip(&cpu).filter(|(a, b)| a != b).count();
	assert!((flipped as f64) < MOST_FLIPPED_SHARE * gpu.len() as f64, "{flipped} pixels differ");
	let (gpu, cpu) = painted_both_ways(COLORED, "stripes", 24, "paint-stripes");
	assert_eq!(gpu.len(), 24 * 16);
	for (pixel, (a, b)) in gpu.iter().zip(&cpu).enumerate() {
		assert!(
			a.iter().zip(b).all(|(a, b)| a.abs_diff(*b) <= CHANNEL_TOLERANCE),
			"pixel {pixel}: {a:?} on the GPU, {b:?} on the CPU"
		);
	}
}

#[test]
fn a_function_wgsl_cannot_compute_paints_on_the_cpu() {
	let program = "width = 6\nheight = 2\nnamed(x, y) := x < count(\"abc\")\n";
	assert!(!lowered(&format!("{program}paint(named, width, height)")).contains("@fragment"));
	assert_eq!(
		painted_rows(&format!("{program}paint(named, width, height)"), "paint-named"),
		[0, 29, 29, 29, 250, 250, 250, 0, 29, 29, 29, 250, 250, 250]
	);
}

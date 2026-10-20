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

/// The WGSL `paint(function, …)` compiles `function` to, as `print shader_of(function)` shows it
fn shader_of(program: &str, function: &str) -> String {
	crate::common::printed(&format!("{program}print shader_of({function})"))
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
	let mandelbrot = shader_of(MANDELBROT, "ink");
	assert!(mandelbrot.contains("@fragment") && mandelbrot.contains("fn warp_escape_time("), "{mandelbrot}");
	let colored = shader_of(COLORED, "stripes");
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
	crate::common::fails_with(&format!("{program}shader_of(named)"), "WGSL cannot compute named");
	assert_eq!(
		painted_rows(&format!("{program}paint(named, width, height)"), "paint-named"),
		[0, 29, 29, 29, 250, 250, 250, 0, 29, 29, 29, 250, 250, 250]
	);
}

/// card compile-shaders: vectors are lists of 2 to 4 numbers, `.+ .*` their element-wise math (decision S2), `p#1` a
/// component, a returned 3-vector of 0…1 the pixel's color
const VECTORS: &str = "use math
width = 32
height = 24
hue(h) := 0.5 .+ 0.5 .* [cos(tau * h), cos(tau * (h + 0.33)), cos(tau * (h + 0.67))]
turned(p, angle) := [cos(angle) * p#1 - sin(angle) * p#2, sin(angle) * p#1 + cos(angle) * p#2]
glow(x, y) := {
	p = turned([x / width - 0.5, y / height - 0.5], 0.5)
	d = norm(p)
	hue(fract(d * 3)) .* (1 - smoothstep(0.3, 0.5, d))
}
";

#[test]
fn a_painted_function_of_vectors_becomes_a_fragment_shader() {
	let shader = shader_of(VECTORS, "glow");
	assert!(shader.contains("@fragment") && shader.contains("fn warp_turned_21(a_p: vec2f"), "{shader}");
	if !has_gpu() {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert_same_colors_both_ways(VECTORS, "glow", (32, 24), "paint-vectors");
}

/// `function` painted on the GPU and on the CPU: the same colors, up to a channel's rounding
fn assert_same_colors_both_ways(program: &str, function: &str, (width, height): (usize, usize), folder: &str) {
	let (gpu, cpu) = painted_both_ways(program, function, width, folder);
	assert_eq!(gpu.len(), width * height);
	for (pixel, (a, b)) in gpu.iter().zip(&cpu).enumerate() {
		assert!(
			a.iter().zip(b).all(|(a, b)| a.abs_diff(*b) <= CHANNEL_TOLERANCE),
			"pixel {pixel}: {a:?} on the GPU, {b:?} on the CPU"
		);
	}
}

/// An item of the program's list: the shader reads it from its values, which hold one vec4f per item
const LEVELS: &str = "width = 16
height = 4
levels = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4]
bars(x, y) := [levels#(x + 1), 0.5, 1 - levels#(x + 1)]
";

#[test]
fn a_painted_function_reads_an_item_of_a_program_list() {
	let shader = shader_of(LEVELS, "bars");
	assert!(shader.contains("values.levels[i32("), "{shader}");
	if !has_gpu() {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert_same_colors_both_ways(LEVELS, "bars", (16, 4), "paint-levels");
}

/// `center()` parses as `(center)`: in a shader it calls the program function of no parameters
const SPOT: &str = "width = 8
height = 8
center() := [width / 2, height / 2]
spot(x, y) := norm([x, y] .- center()) < 3
";

#[test]
fn a_painted_function_calls_a_function_of_no_parameters() {
	let shader = shader_of(SPOT, "spot");
	assert!(shader.contains("fn warp_center() -> vec2f") && shader.contains("- warp_center()"), "{shader}");
	if !has_gpu() {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert_same_colors_both_ways(SPOT, "spot", (8, 8), "paint-spot");
}

/// round takes a half to the even neighbour on the CPU and in a shader alike: 0.5 → 0, 1.5 → 2, 2.5 → 2
const STEPS: &str = "width = 8
height = 2
steps(x, y) := round(x / 2 + 0.5) % 2
";

#[test]
fn a_painted_function_rounds_halves_to_even() {
	assert!(shader_of(STEPS, "steps").contains("round(("));
	if !has_gpu() {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert_same_colors_both_ways(STEPS, "steps", (8, 2), "paint-steps");
}

/// Note (lib/sound.warp, loaded by melody_samples) defines plus: `x + 1` reads `if x is Note then x.plus(1) else x + 1`,
/// of which the shader computes the built-in operator, no value on the GPU being a Note
#[test]
fn a_painted_function_of_a_program_with_notes_becomes_a_fragment_shader() {
	let program = "use sound
song = melody_samples([C4], 0.25)
near(x, y) := x + 1 < 2
";
	assert!(shader_of(program, "near").contains("@fragment"));
}

/// card compile-shaders step 4, decision S1: `p.x`, `c.zxy` are WGSL's swizzles of the vector, the same colors as the
/// CPU's components
const SWIZZLES: &str = "use math
width = 32
height = 24
turned(p, angle) := [cos(angle) * p.x - sin(angle) * p.y, sin(angle) * p.x + cos(angle) * p.y]
glow(x, y) := {
	p = turned([x / width - 0.5, y / height - 0.5], 0.5)
	c = [0.5 + 0.5 * sin(p.x * 6), 0.5 + 0.5 * cos(p.y * 6), 0.5]
	c.zxy .* (1 - smoothstep(0.3, 0.5, norm(p.yx)))
}
";

#[test]
fn swizzles_become_the_shaders_swizzles() {
	let shader = shader_of(SWIZZLES, "glow");
	assert!(shader.contains("@fragment") && shader.contains(".zxy") && shader.contains("v_p.x"), "{shader}");
	if !has_gpu() {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert_same_colors_both_ways(SWIZZLES, "glow", (32, 24), "paint-swizzles");
}

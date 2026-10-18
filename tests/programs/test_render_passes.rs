// card gpu-multipass: `waves = render(f, width, height)` (alias texture) draws f into an image on the GPU before the
// paint whose function reads it as `waves(at)`, at a vector of 2 or two numbers; on the CPU `waves(at)` is f at the
// nearest pixel, so both paint the same (notes/gpu.md "Several passes")
#![cfg(feature = "native")]
use super::test_paint_function::{assert_same_colors_both_ways, has_gpu, lowered};

const SHIFTED: &str = "width = 8
height = 8
gradient(x, y) := [x / width, y / height]
waves = render(gradient, width, height)
shifted(x, y) := {
	at = [x + 2, y]
	[waves(at).x, waves(x, y + 1).y, 0]
}
";

#[test]
fn a_painted_function_reads_an_image_rendered_before() {
	let shader = lowered(&format!("{SHIFTED}paint(shifted, width, height)"));
	assert!(shader.contains("warp_image_waves(v_at).x") && shader.contains("shader:"), "{shader}");
	if !has_gpu() {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert_same_colors_both_ways(SHIFTED, "shifted", (8, 8), "render-shifted");
}

const TEXTURED: &str = "width = 6
height = 4
stripes(x, y) := x % 2
pattern = texture(stripes, 3, 3)
read(x, y) := pattern(x, y) < 0.5
";

#[test]
fn texture_is_render() {
	if !has_gpu() {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert_same_colors_both_ways(TEXTURED, "read", (6, 4), "render-textured");
}

#[test]
fn an_image_wgsl_cannot_render_paints_on_the_cpu() {
	let program = "width = 4\nheight = 2\nnamed(x, y) := x < count(\"abc\")\nimage = render(named, 4, 2)\nread(x, y) := image(x, y)\n";
	assert!(!lowered(&format!("{program}paint(read, width, height)")).contains("@fragment"));
}

/// The paint's values carry the image's map, made where the paint runs: f's numbers are each frame's, not those of the
/// `render` statement
const ANIMATED: &str = "width = 4
height = 4
global time = 0.0
moving(x, y) := fract(x / width + time)
waves = render(moving, width, height)
read(x, y) := waves(x, y) < 0.5
";

#[test]
fn an_image_renders_with_each_frames_numbers() {
	let lowered = lowered(&format!("{ANIMATED}for frame in 0..2 {{ time = frame / 4; paint(read, width, height) }}"));
	assert!(lowered.contains("time:time}"), "{lowered}");
}

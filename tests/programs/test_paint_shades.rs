// card graphics-shade: what a painted pixel shows (0 paper, below COLOR_FROM ink, else 0xAARRGGBB) is defined in
// src/graphics/pixels.rs; the playground's host and lib/draw.warp's paper repeat it, and must say the same
use warp::graphics::pixels::{COLOR_FROM, INK, PAPER};

const HOST_MEDIA: &str = include_str!("../../web/playground/host-media.js");
const DRAW: &str = include_str!("../../lib/draw.warp");

/// The value after `prefix` up to the end of its line: `const PAINT_INK = 29;` → 29, `2 ** 24` → 16777216
fn number_after(source: &str, prefix: &str) -> u64 {
	let line = source.lines().find_map(|line| line.strip_prefix(prefix)).unwrap_or_else(|| panic!("no {prefix}"));
	let value = line.trim().trim_end_matches(';');
	match value.split_once(" ** ") {
		Some((base, power)) => base.parse::<u64>().unwrap().pow(power.parse().unwrap()),
		None => match value.strip_prefix("0x") {
			Some(hex) => u64::from_str_radix(hex, 16).unwrap(),
			None => value.parse().unwrap(),
		},
	}
}

#[test]
fn the_hosts_shade_pixels_as_pixels_rs_says() {
	assert_eq!(number_after(HOST_MEDIA, "const PAINT_INK = "), INK as u64);
	assert_eq!(number_after(HOST_MEDIA, "const PAINT_PAPER = "), PAPER as u64);
	assert_eq!(number_after(HOST_MEDIA, "const PAINT_COLOR_FROM = "), COLOR_FROM);
	assert_eq!(number_after(DRAW, "paper = "), 0xFF000000 | (PAPER as u64 * 0x010101));
}

#[cfg(feature = "native")]
const HOST: &str = include_str!("../../web/playground/host.js");

/// The number after `name: ` in a line of `source`: `view_width: 640,` → 640
#[cfg(feature = "native")]
fn entry_after(source: &str, name: &str) -> i64 {
	let prefix = format!("{name}: ");
	let at = source.find(&prefix).unwrap_or_else(|| panic!("no {prefix}")) + prefix.len();
	source[at..].chars().take_while(char::is_ascii_digit).collect::<String>().parse().unwrap()
}

// card paper-shape: the default view (system_values.rs NATIVE_VIEW, host.js VIEW_DEFAULTS) and draw_shape's clear
// kind (gpu_canvas.rs CLEAR, lib/draw.warp clear_shape) are written twice, and must say the same
#[cfg(feature = "native")]
#[test]
fn the_view_and_the_clear_shape_agree_everywhere() {
	let (width, height) = warp_runtime::system_values::native_view();
	assert_eq!((entry_after(HOST, "view_width"), entry_after(HOST, "view_height")), (width, height));
	assert_eq!(number_after(DRAW, "clear_shape = "), warp::graphics::gpu_canvas::CLEAR as u64);
}

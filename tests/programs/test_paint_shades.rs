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

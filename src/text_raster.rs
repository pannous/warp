//! Text set in a real font natively (card paint-text): `label` of `use draw` asks the host word text_coverage for the
//! words' coverage and blends it into its canvas, the same pixels paint shows. The playground sets them with the
//! browser's canvas 2D fillText (web/playground/host.js textCoverage), here a system sans-serif font through ab_glyph.
//! WARP_FONT names another font file; a machine without any of the fonts is a loud error.

use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use std::sync::OnceLock;

const FONT_VARIABLE: &str = "WARP_FONT";
/// the sans-serif fonts looked for, the first found used: what browsers show for sans-serif on each system
const SYSTEM_FONTS: [&str; 8] = [
	"/System/Library/Fonts/Helvetica.ttc",
	"/System/Library/Fonts/Supplemental/Arial.ttf",
	"/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
	"/usr/share/fonts/TTF/DejaVuSans.ttf",
	"/usr/share/fonts/dejavu/DejaVuSans.ttf",
	"/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
	"/usr/share/fonts/liberation-sans/LiberationSans-Regular.ttf",
	"C:\\Windows\\Fonts\\arial.ttf",
];

/// [width, height, then how much each pixel is covered from 0 to 255, row by row] of the words set `size` pixels per em,
/// on one line from the top left, its height the font's ascent and descent
pub fn coverage(words: &str, size: f32) -> Result<Vec<u32>, String> {
	let font = font()?;
	let font = font.as_scaled(PxScale::from(size * font.height_unscaled() / font.units_per_em().unwrap_or(1.0)));
	let mut glyphs = Vec::new();
	let mut caret = 0.0;
	let mut previous = None;
	for character in words.chars() {
		let id = font.glyph_id(character);
		if let Some(before) = previous {
			caret += font.kern(before, id);
		}
		glyphs.push(id.with_scale_and_position(font.scale(), ab_glyph::point(caret, font.ascent())));
		caret += font.h_advance(id);
		previous = Some(id);
	}
	let (width, height) = (caret.ceil().max(1.0) as usize, (font.ascent() - font.descent()).ceil().max(1.0) as usize);
	let mut covered = vec![0u32; width * height];
	for outlined in glyphs.into_iter().filter_map(|glyph| font.outline_glyph(glyph)) {
		let bounds = outlined.px_bounds();
		outlined.draw(|x, y, amount| {
			let (column, row) = (bounds.min.x as i64 + i64::from(x), bounds.min.y as i64 + i64::from(y));
			if (0..width as i64).contains(&column) && (0..height as i64).contains(&row) {
				let cell = &mut covered[row as usize * width + column as usize];
				*cell = (*cell + (amount.clamp(0.0, 1.0) * 255.0).round() as u32).min(255);
			}
		});
	}
	Ok([width as u32, height as u32].into_iter().chain(covered).collect())
}

/// The font, read once
fn font() -> Result<&'static FontVec, String> {
	static FONT: OnceLock<Result<FontVec, String>> = OnceLock::new();
	FONT.get_or_init(|| {
		let path = std::env::var(FONT_VARIABLE).ok().or_else(|| SYSTEM_FONTS.iter().find(|path| std::path::Path::new(path).exists()).map(|path| path.to_string()))
			.ok_or_else(|| format!("no font found: install DejaVu Sans, or name a .ttf in {FONT_VARIABLE} (looked in {})", SYSTEM_FONTS.join(", ")))?;
		let bytes = std::fs::read(&path).map_err(|failure| format!("cannot read the font {path}: {failure}"))?;
		FontVec::try_from_vec_and_index(bytes, 0).map_err(|failure| format!("the font {path}: {failure}"))
	}).as_ref().map_err(Clone::clone)
}

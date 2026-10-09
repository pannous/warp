# Drawing: `use draw` and paint

One pipeline paints, on the GPU or without one: `paint(pixels, width, height)` shows a list of 0xAARRGGBB numbers,
natively in a window (a PNG with WARP_NO_WINDOW or CI, src/paint.rs) and in the playground on a canvas
(playground.js showPaintings, ⛶ full screen). `paint(shader, w, h, values)` renders WGSL to those pixels first
(notes/web_framework.md "Values", "Holes").

`use draw` (lib/draw.warp) keeps a canvas of such pixels: `canvas(w, h)`, `clear`, `dot`, `rect`, `circle`, `line`,
colors (`red`, `rgb`, `hsv`, `with_alpha`), and `show()` paints it. tests/programs/test_draw.rs.

Text (card paint-text, 2026-10-09, warp-web): `label(x, y, words, color, size = 12)` sets words in a real sans-serif
font, `size` pixels per em, from their top left corner (the line's height is the font's ascent + descent), smooth edges
mixed with what is below. The host word `text_coverage(words, size)` gives [width, height, coverage 0–255 row by row]:
in the playground the browser's canvas 2D fillText on an OffscreenCanvas (host.js textCoverage, also in the run's
Worker), natively ab_glyph over a system font (src/text_raster.rs: Helvetica on a Mac, DejaVu Sans or Liberation Sans
on Linux, Arial on Windows, WARP_FONT names another; none is a loud error). label blends it into canvas_pixels, so
show()/paint stays the one pipeline. `text` cannot be the word: "text is a type". A first version with a 3×5 pixel
font was dropped (user: "the classical canvas"). samples/paint_text.warp, tests/programs/test_draw_label.rs.

# Drawing: `use draw` and paint

One pipeline paints, on the GPU or without one: `paint(pixels, width, height)` shows a list of 0xAARRGGBB numbers,
natively in a window (a PNG with WARP_NO_WINDOW or CI, src/graphics/paint.rs) and in the playground on a canvas
(playground.js showPaintings, ⛶ full screen). `paint(shader, w, h, values)` renders WGSL to those pixels first
(notes/web_framework.md "Values", "Holes"). Without a size, `paint(x)` / `paint(x, values)` take the program's
`width` and `height` variables, else view_width × view_height (src/graphics/shader_holes.rs with_sizes).

`use draw` (lib/draw.warp) keeps a canvas of such pixels: `canvas(w, h)`, `clear`, `dot`, `rect`, `circle`, `line`,
colors (`red`, `rgb`, `hsv`, `with_alpha`), and `show()` paints it. tests/programs/test_draw.rs.

Text (card paint-text, 2026-10-09, warp-web): `label(x, y, words, color, size = 12)` sets words in a real sans-serif
font, `size` pixels per em, from their top left corner (the line's height is the font's ascent + descent), smooth edges
mixed with what is below. The host word `text_coverage(words, size)` gives [width, height, coverage 0–255 row by row]:
in the playground the browser's canvas 2D fillText on an OffscreenCanvas (host.js textCoverage, also in the run's
Worker), natively ab_glyph over a system font (src/graphics/text_raster.rs: Helvetica on a Mac, DejaVu Sans or Liberation Sans
on Linux, Arial on Windows, WARP_FONT names another; none is a loud error). label blends it into canvas_pixels, so
show()/paint stays the one pipeline. `text` cannot be the word: "text is a type". A first version with a 3×5 pixel
font was dropped (user: "the classical canvas"). samples/paint_text.warp, tests/programs/test_draw_label.rs.

## On the GPU (card gpu-draw)
A second implementation of `use draw`, chosen by the system value `gpu_drawing` (crates/warp-runtime system_values,
set by paint.rs allow_windows): with a window on by default, `WARP_DRAW=gpu|cpu` forces either; an unknown value warns.
- lib/draw.warp: clear, dot, rect, circle call the host word `draw_shape(kind, a, b, c, d, color)`; line and label go
  through dot; canvas() allocates no pixel list; show() calls `draw_show(width, height)`.
- src/graphics/gpu_canvas.rs gathers the shapes (kind u32, color u32, 4×f32) and draws them instanced with one WGSL shader into
  an Rgba8Unorm canvas texture that stays on the GPU between shows (alpha blending as the CPU's mixed(); a circle
  colors the pixel centers within its radius, a rect cells cell(x)..cell(x+width), so both paths give the same PNG).
- In a window only the shapes since the last show cross the pipe (paint_window.rs `shapes_message`, marked by
  u32::MAX where a frame has its width); the paint-window process owns the canvas and blits it to the surface.
  Headless, an offscreen canvas is read back and written as paint's PNG.
- The playground keeps the CPU path (gpu_drawing is 0 there); its draw_shape/draw_show stubs are in host-gpu.js.
- Checks: tests/programs/test_gpu_draw.rs (GPU vs CPU pixels), probes/gpu_draw/compare.py <warp> scene.warp.

## Saving a picture (card save-painting)
`save "painting.png"`, `save canvas as: "painting.png"`, `save(canvas, "painting.png")`: the canvas as drawn so far as a
PNG (decision save-sound: one `save`, overloaded by type). src/lowering/words/saved_words.rs makes the word `canvas` as
save's argument lib/draw.warp's `Picture()` (as `sound` is sound.warp's Recording); its save calls the host word
`save_picture(canvas_pixels, width, height, file)`. Natively paint.rs save_png writes the file; on the GPU the pixels
are in the window's process, so gpu_canvas.rs keeps the shapes since the last clear (UNMIRRORED) and snapshot draws them
into the in-process OFFSCREEN canvas and reads it back (headless show uses the same). The playground shades the pixels
as paint does (host-media.js), and the page makes a PNG of them (playground.js savePicture), downloads it at once and
offers it beside the wasm link. tests/programs/test_save_painting.rs.

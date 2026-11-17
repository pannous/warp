# Crisp graphics (card crisp-graphics)

How `use draw` (lib/draw.warp) stays sharp on any display, and the frame budget.

- **Display pixels.** `pixel_scale` = display_dpi / 96, rounded, at least 1 (a Retina display 2; measured as each canvas is
  made). Programs keep canvas units; the canvas is drawn at `canvas × pixel_scale` device pixels.
- **paint_scale** (system value, set before a paint): the next painting's pixels to one point across. The hosts show the
  painting at its size in points and tell the pointer in points: the playground (host.js → canvas.js paintingElement,
  pointOn), the native window (window.warp host_window_layer). It holds for one painting only, so a shader painted after
  a scaled canvas is not misread.
- **Smooth edges.** Lines are capsules (round ends) and circles discs, each pixel covered by `edge(distance)`: the share of
  a one-pixel ramp. A polyline skips each joint's second round end, so translucent rings show no dots.
- **GPU first.** Where a window or the page shows the paintings and a GPU draws (system value gpu_drawing: native host
  when a window can open, the playground while the page shows paintings), the shapes go to one instanced shader
  (shape_shader): three vec4s a shape, one quad each, blended in order, the frame before as the first quad
  ($previous). Clear drops the shapes it covers. Text is a coverage list read by a text quad. PGA: 7 ms a frame against
  200 ms on the CPU at 2×.
- **CPU fallback.** With no window (`--no-window`, `warp test`, a test page) the pixels are drawn here, so canvas_pixels
  can be read. A CPU canvas whose frames, clear to show, take over 33 ms three times in a row at more than one pixel a
  point drops to one pixel a point (said once).
- **paint(shader) natively** is shaded at the window's own pixels (gpu.warp shade_fitted), its `$mouse` in those pixels.
- Still open: natively display_dpi is the driver's default 96 (core/native/driver.warp answers it before the native host),
  so a native CPU canvas stays at one pixel a point. The GPU path doesn't need it: the host fits the window's pixels.

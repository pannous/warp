# Window, display and canvas sizes (decision canvas-size)

`window.width/height`, `display.width/height` and `canvas.width/height` are properties: read like fields, written
like them where the platform can resize. `canvas(w, h)` stays; a bare `width`/`height` the program doesn't bind is
the canvas's, with a hint naming `canvas.width` and the other two.

| property | reads | a write |
|---|---|---|
| `canvas.width` | `canvas_width` of lib/draw.warp | `canvas(it, canvas_height)`: a new blank canvas, as an HTML canvas |
| `window.width` | system value `window_width` | `set_system_value("window_width", it)` |
| `display.width` | system value `display_width` | compile-time error: a program cannot resize the display |

Mechanism: src/lowering/definitions/property_accessors.rs, a table of object, property, getter, setter (warp code, `it` the
value written) and the module it needs. Runs in SUGAR_PASSES right after the lookups are normalized (`a of b`, `(p).a`),
before any pass reads a field write; a property brings its module (`use draw`) like std_aliases does. `+=` and its
kin read, then write. A program binding the object or word keeps its own (`canvas = {width: 9}`, `f(width) := …`,
`for width in …`); field names (`{width: 5}`, `p.width`, named arguments) are never reads. The window's getters are
system values, so `whenever window.width > 800 {…}` listens like `battery`.

Native (crates/warp-window over SDL3, crates/warp-gpu over wgpu): a window the program gave no size opens at the
largest whole multiple of its first painting that fits 70% of the screen (a 320×200 painting no longer opens tiny). It
resizes freely and goes full screen; libwarp_gpu configures its surface at the Metal layer's drawableSize (SDL keeps
that at the window's pixels) and draws each painting at its own size, then scales it in whole, aspect kept, centered
(nearest pixel, black bars). The pointer is mapped through the same letterbox. Pitfall: a surface configured at the
painting's size while SDL resizes the drawable shows the painting 1:1 in the corner with smeared edge pixels; forcing
the drawable back to the painting's size instead lets Core Animation stretch it (distorted in full screen).
display: CoreGraphics main display on macOS (points), `xrandr --current` elsewhere.

Playground (web/playground/host.js `set_system_value`): the window is the frame the run's paintings show in, in CSS
pixels, the view's size (output pane) until the run sets it; the page sizes the shown paintings and those to come
(canvas.js resizeWindow, reset at each run). display is `screen.width/height`, told by the page with the other
system values.

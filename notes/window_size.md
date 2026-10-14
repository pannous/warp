# Window, display and canvas sizes (card make-width, decision canvas-size)

`window.width/height`, `display.width/height` and `canvas.width/height` are properties: read like fields, written
like them where the platform can resize. `canvas(w, h)` stays; a bare `width`/`height` the program doesn't bind is
the canvas's, with a hint naming `canvas.width` and the other two.

| property | reads | a write |
|---|---|---|
| `canvas.width` | `canvas_width` of lib/draw.warp | `canvas(it, canvas_height)`: a new blank canvas, as an HTML canvas |
| `window.width` | system value `window_width` | `set_system_value("window_width", it)` |
| `display.width` | system value `display_width` | compile-time error: a program cannot resize the display |

Mechanism: src/lowering/words/module_properties.rs, a table of object, property, getter, setter (warp code, `it` the
value written) and the module it needs. Runs in SUGAR_PASSES right after the lookups are normalized (`a of b`, `(p).a`),
before any pass reads a field write; a property brings its module (`use draw`) like std_aliases does. `+=` and its
kin read, then write. A program binding the object or word keeps its own (`canvas = {width: 9}`, `f(width) := …`,
`for width in …`); field names (`{width: 5}`, `p.width`, named arguments) are never reads. The window's getters are
system values, so `whenever window.width > 800 {…}` listens like `battery`.

Native (crates/warp-runtime/src/system_values.rs `write`): the window size is kept in points, as last reported by the
viewer (`size w h` line on its stdout at open and on each resize) or written by the program, else the view's 640×480.
A write calls the warp binary's resizer (src/graphics/paint.rs allow_windows → paint_window::resize): before a viewer
runs the size waits and goes first to the viewer as it starts; a running one gets `[RESIZE_MARK][w][h]`. The viewer
opens at the asked size and keeps it: frames of another size then scale into it instead of resizing the window.
Headless (WARP_NO_WINDOW, tests) there is no resizer: the written size is only read back. display: CoreGraphics main
display on macOS (points), `xrandr --current` elsewhere.

Playground (web/playground/host.js `set_system_value`): the window is the frame the run's paintings show in, in CSS
pixels, the view's size (output pane) until the run sets it; the page sizes the shown paintings and those to come
(canvas.js resizeWindow, reset at each run). display is `screen.width/height`, told by the page with the other
system values.

# The native window in warp (decision window-in-warp, card window-ffi-replace)

The native warp's window is lib/native_window.warp, a warp module over SDL3 through FFI (lib/native/SDL3.warp). No C
wrapper: crates/warp-window/warp_window.c is gone.

## How the driver reaches it
- modules/native_window.wasm is built by the rebuild (scripts/rebuild.sh) like every standard module; nothing compiles
  lib/ at run time (no-lib-reads).
- The driver links it lazily at the first question (core/native/driver.warp `lazily_linked`, the same path as unicode for
  canonical_text), so a program that never asks about the window never loads it nor SDL3.
- Its exports are the hook: `host_owns(name)` / `host_value(name)` / `host_value_set(name, value)` for system values
  (window_open, pointer, keys, mouse, dark mode, window size while open), `host_window_layer(w, h)` opens the window
  and hands libwarp_gpu its CAMetalLayer, `host_run_ended()` holds a still picture open, `host_text_coverage` answers
  the host word text_coverage. A host word `name` the driver doesn't know is answered by an export `host_<name>`.
- The driver's headless guard (headless_checked: a program's window-opening C import refused under WARP_NO_WINDOW) is
  not applied to modules the driver links itself (`guarded` no); the module asks WARP_NO_WINDOW / CI itself and stays
  windowless when SDL3 is missing (`library_function("SDL3", "SDL_Init") == 0`).
- Records made while linking inside a host word survive the host-word arena reset through `arena_kept()`
  (lib/native/foreign.c).

Named native_window, not window: `window` is the prelude's property object (`window.width`).

## Struct fields by offset (lib/c.warp field_*)
A C struct (SDL_Event, a FreeType glyph slot, a CGRect) is calloc'd memory the program holds as a handle and reads with
`field_u32(event, 0)`, `field_f32(event, 28)` …: no reflection, no name lookup; the offset is a constant at the call
site. Each read is one FFI call (the struct lives in C memory, outside the module's linear memory); a bulk copy into
linear memory is the optimization if it ever matters.

## Text (text_coverage)
macOS: CoreText over FFI (CTLineDraw into an alpha-only CGBitmapContext), native look, no dependency.
Linux, or WARP_FONT set: FreeType over FFI (libfreetype.so.6), glyph slot fields by offset. Both give the same
coverage list (width, height, rows) through `coverage_list`.

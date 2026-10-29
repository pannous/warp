# Reusing existing work for windows, graphics and the GPU (card reuse-graphics, user policy reuse-giants 2026-10-29)

What the native warp does today: `paint` writes a PNG (core/native/driver.warp, its own PNG writer). Nothing else
graphical is answered since the Rust harness went: no window, no events beyond Key.none headless, no `draw_shape` /
`draw_show` GPU canvas, no `text_coverage` (so `label`), no `gpu_compute` / `gpu_render` / shader paint. The Rust
harness had them from winit, wgpu, ab_glyph and the png crate (rust-harness:src/graphics/). The playground keeps its
own: canvas 2D, WebGPU, OffscreenCanvas text. So the question is which C libraries replace the Rust crates, bound
the way lib/ binds C (`from 'library'` lines, a small wrapper where the API is structs or callbacks).

## Verdicts

| Need | Take | Binding | License | We stop building |
|---|---|---|---|---|
| Window, input, events, high-DPI, clipboard, frame pacing | **SDL3** | C ABI. Events are a union struct, so a small wrapper (libwarp_window, as libwarp_sound) flattens SDL_PollEvent into ints: kind, key, x, y, button, wheel. lib/native/SDL2.warp shows the plain calls already work | zlib | the paint-window process, winit's event loop, our key-code table (SDL scancodes → lib/keys.warp `Key` cases) |
| Showing pixels (`paint`, the CPU canvas of `use draw`) | SDL3's `SDL_Renderer`: one streaming texture updated from the pixel list per frame | same wrapper | zlib | the window blit of paint_window.rs |
| GPU: WGSL shaders, `gpu_compute`, `gpu_render`, shader `paint`, the GPU canvas of `use draw` | **wgpu-native** (webgpu.h, the C API of the wgpu the Rust harness used) | C ABI with callbacks and descriptor structs, so a wrapper (libwarp_gpu) offering a few calls: compute(wgsl, buffers) → buffers, render(wgsl, w, h, values) → pixels or onto the SDL3 window's surface (a CAMetalLayer / X11 / Wayland handle) | MIT or Apache-2.0 | nothing of ours is lost: WGSL stays the shader language on both sides, the same as WebGPU in the playground |
| Text (`label`, `text_coverage`) | **stb_truetype** (one header) over a system font file | compiled into libwarp_window | public domain / MIT | ab_glyph; the coverage API stays as it is (width, height, coverage rows), the font lookup (Helvetica, DejaVu Sans, Arial, WARP_FONT) stays ours |
| Reading images (`image(path)`: PNG, JPEG, GIF, BMP) | **stb_image** | same wrapper or its own tiny one | public domain / MIT | any decoder of ours |
| Writing PNG | keep ours (driver.warp already writes it, headless, no dependency); stb_image_write if the driver should lose it | — | — | — |

## Not taken
- **raylib** (zlib): window, input, 2D/3D drawing, text and audio (it embeds miniaudio) in one easy C API; good for
  games, and lib/native/raylib.warp stays as a user library. Not the basis: OpenGL only (no WGSL, no compute), and its
  drawing model would be a second one beside `use draw` and WebGPU.
- **GLFW** (zlib): window and input only, no renderer or text; SDL3 is the superset at the same binding cost.
- **winit** through a C-ABI shim: the Rust we just removed, back as a crate to build per platform; SDL3 covers it in C.
- **SDL_GPU** (inside SDL3): a modern GPU API, but its shaders are SPIR-V/MSL/DXIL, not WGSL; it would need naga or
  tint in between, which wgpu-native already is.
- **Dawn** (BSD-3): Chrome's WebGPU, same webgpu.h, but a heavy C++ build; wgpu-native ships prebuilt releases.
- **Skia, Blend2D, ThorVG, NanoVG**: anti-aliased vector drawing and SVG; worth a look when `use draw` wants vector
  paths and gradients. Until then the pixel canvas stays plain warp, so the playground paints the same pixels.
- **sokol** (zlib): small headers for window and GPU, but its own shader toolchain (sokol-shdc).

## Shape
Two wrapper libraries beside libwarp_wasmtime, built by scripts/wasmtime-c-api.sh as libwarp_sound is (branch
sound-plays): libwarp_window (SDL3, stb_truetype, stb_image) and libwarp_gpu (wgpu-native). SDL3 and wgpu-native
come as their prebuilt releases (SDL3 also from brew/apt), fetched at a pinned version and checked, as miniaudio is.
lib/frames.warp, lib/draw.warp and lib/keys.warp call them `from 'warp_window'` / `from 'warp_gpu'`; headless
(WARP_NO_WINDOW, CI) nothing opens, as today. The driver keeps no graphics of its own but the PNG writer. As with
sound, a missing library should fail only when called (the core question on branch sound-plays), so a headless
install without SDL3 still runs every program that does not open a window.

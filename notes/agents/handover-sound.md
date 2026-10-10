# Handover: warp-sound (2026-10-10)

Retired with no sound or graphics cards left on the board; the next sound/paint worker starts here.

## Open
- native-system (branch native-mouse, Integrator batch 137): native mouse_x / mouse_y / mouse_down from the paint
  window, samples/finger_paint.warp. Close with `todo done native-system <tip>` once it is on main, if not done yet.
- max-typed (warp-numbers): `r += max(0.0, x - l[b])` in a loop over a comprehension list fails with "extremum
  argument 1 is a float where an exact Int is expected". samples/gpu_visualizer.warp works around it with
  `if level > levels[band] { rise += level - levels[band] }`; warp-numbers drops the workaround with its fix.
- Ideas, no card yet: the native window size as view_width / view_height once a paint window exists (the viewer
  could report its inner size on its `input` stdout line); $key holds only one key (the last pressed).

## Where things are
- Shader holes and built-ins: src/shader_holes.rs (BUILTIN_HOLES), src/gpu.rs with_builtin_values, the browser side
  web/playground/host-gpu.js builtinValues and canvas.js (pointer + key in the shared buffer). notes/gpu.md
  "Shader built-ins" and "Painting a shader every frame".
- Paint window: src/paint_window.rs, a separate `warp paint-window` process fed RGBA frames on stdin, writing
  `input x y down key` lines on stdout; warp-runtime system_values.rs window_input keeps them (system values and
  shader built-ins both read it). Frames of 1024+ px across show pixel for pixel (shown_size).
- RGBA fast path: gpu::render_rgba → paint::paint_rgba → paint_window::rgba_frame (no Node list, ~5 ms at 720p debug).
- Default canvas: lib/draw.warp sizes it from system values view_width / view_height (the playground's output pane
  via playground.js viewSize; natively 640×480). notes/web_playground.md "The default canvas is the output pane".
- Sound: src/sound.rs (WAV files, the audio clock); spectrum(samples, seconds, bands) for visualizers.

## Probes
- probes/gpu/paint_cost.warp, render_speed.warp, web_speed.py: paint and render timing.
- probes/sound/sound_words.warp: the sound words. probes/paint_window.sh: the viewer process.
- probes/webgpu/: gpu_compute thresholds. tests/web/test_playground_paint.rs runs host.js under node (a harness
  for playground host behaviour without a browser; compile with `pipeline::for_a_page`).

## Quirks
- Probes with paint must run under WARP_NO_WINDOW=1: otherwise windows pop up and sound plays on the user's Mac.
  Headless, sound only prints `sound N s: <wav path>` and paint writes PNGs.
- The PNG folder $TMPDIR/warp-paint is shared by every session: filter frames by size/mtime when inspecting.
- A headless 1080p animation is bound by PNG writing (deflate is in fast mode); the window path isn't.
- Shader built-ins are skipped when the program assigns a variable of that name (gpu_visualizer keeps its own
  `time`, the song's clock).
- Hints: `sleep 16ms` (a unit on its number), `levels#13` for indexing is suggested, not required.

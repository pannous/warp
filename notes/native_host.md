# The native host module in warp (decision window-in-warp; cards window-ffi-replace, warp-sound-c)

What the playground answers in JavaScript beyond the driver's own words, the native warp answers in warp:
lib/host/native_host.warp, one module of its parts, native_host/window.warp (SDL3's window, CoreText/FreeType text),
native_host/sound.warp (SDL3's audio, AudioToolbox / libmpg123 decoding, CoreMIDI), native_host/webview.warp (WKWebView)
and native_host/gpu.warp (wgpu-native), over C libraries by plain FFI. No C wrappers: crates/warp-window,
crates/warp-sound, crates/warp-webview and crates/warp-gpu are gone.

## How the driver reaches it
- modules/native_host.wasm is built by the rebuild (scripts/rebuild.sh) like every standard module (it is in
  rewrites.warp std_module_names); nothing compiles lib/ at run time (no-lib-reads). Its parts are files it uses by
  path (`use "native_host/window.warp"`), so they are one module, one link, sharing on_mac, quiet_run and cf_text.
- The driver links it lazily at the first question (core/native/driver.warp `lazily_linked`, the same path as unicode for
  canonical_text), so a program that never asks about the window or sound never loads it, SDL3 nor a decoder.
- Its exports are the hook: `host_owns(name)` / `host_value(name)` / `host_value_set(name, value)` for system values
  (window_open, pointer, keys, mouse, dark mode, window size while open), `host_window_layer(w, h)` opens the window
  and hands gpu.warp its CAMetalLayer, `host_run_ended()` holds a still picture open and waits till the sounds are
  heard out. A host word `name` the driver doesn't know is answered by an export `host_<name>`: text_coverage,
  sound_played, sound_file_played, sound_file_decoded, midi_sent, midi_ports.
- The driver's headless guard (headless_checked: a program's window-opening C import refused under WARP_NO_WINDOW) is
  not applied to modules the driver links itself (`guarded` no); the module asks WARP_NO_WINDOW / CI itself
  (quiet_run: no window, no sound, no MIDI) and stays windowless and silent when SDL3 is missing.
- Records made while linking inside a host word survive the host-word arena reset through `arena_kept()`
  (lib/native/foreign.c).
- A standard module's own `import f from 'c'` is claimed as its foreign lines are (extensions/c_imports
  c_declarations_used); before, a module importing from 'c' never prebuilt.

Named native_host, not window or sound: those are the prelude's property object and the sound module.

## Struct fields by offset (lib/c.warp field_* and set_field_*)
A C struct (SDL_Event, a FreeType glyph slot, an AudioStreamBasicDescription, an AudioBufferList) is calloc'd memory the
program holds as a handle, read with `field_u32(event, 0)`, `field_f32(event, 28)`, written with `set_field_i32`,
`set_field_f64`, `set_field_pointer` …: no reflection, no name lookup; the offset is a constant at the call site. Each
access is one FFI call (the struct lives in C memory, outside the module's linear memory). A decoded sound file is
read value by value so (~1 call a sample); a bulk copy into linear memory is the optimization if it ever matters.
`field_text(memory, 0)` reads a C string there (CFStringGetCString's buffer).

## Text (text_coverage)
macOS: CoreText over FFI (CTLineDraw into an alpha-only CGBitmapContext), native look, no dependency.
Linux, or WARP_FONT set: FreeType over FFI (libfreetype.so.6), glyph slot fields by offset. Both give the same
coverage list (width, height, rows) through `coverage_list`.

## Sound
- Playing: a stream per sound to the default device (SDL_OpenAudioDeviceStream, SDL_PutAudioStreamData of the
  little-endian 16-bit bytes), SDL mixing them; streams played out are closed at the next sound; the run's end waits
  till every queue is empty plus DEVICE_TAIL_MS. Without SDL3 or a device, lib/sound.warp falls back to the system's
  player with a WAV, as before.
- Decoding (`sample`, `sound_seconds`, `spectrum`, `play "song.mp3"`): [rate, channels, values…], lib/sound.warp
  caching each file once a run. macOS: AudioToolbox ExtAudioFile (wav, mp3, flac, aac, m4a), converting rate and
  channels itself and leaving out an MP3's encoder delay and padding (decision mp3-true-length). Elsewhere: a WAV by
  SDL_LoadWAV (made 16-bit by SDL_ConvertAudioSamples), an MP3 by libmpg123 (missing: refused naming the package),
  then remixed and resampled linearly in warp as the page does.
- MIDI out: CoreMIDI over FFI (client, output port, a virtual source "warp" when there is no output); its endpoint
  names by kMIDIPropertyDisplayName, a CFString global read through library_function + field_pointer. Linux: not yet.
- The page answers the same host words in web/playground/host-sound.js soundWords.

## The GPU (gpu.warp, card warp-gpu-source, 2026-11-08)
crates/warp-gpu/warp_gpu.c (918 lines over wgpu-native's webgpu.h, statically linked into libwarp_gpu) became the part
native_host/gpu.warp over libwgpu_native, the wgpu-native release's shared library, which scripts/wasmtime-c-api.sh
installs beside warp. Its mechanisms:
1. Callbacks (card ffi-callbacks): lib/callbacks.warp `c_callback(f, types)` hands C one of 16 foreign.c trampolines
   (six integer registers each). gpu.warp makes three once (an adapter's or device's answer, an error scope's, a
   buffer's mapping); each writes the answer into globals, and the call polls wgpuInstanceProcessEvents (and
   wgpuDevicePoll) till it is there.
2. Structs by value: no new parameter kind on ARM64 (AAPCS64, macOS and Linux): a composite over 16 bytes goes as a
   pointer to a caller's copy, so a WGPU*CallbackInfo (40 bytes) is a `handle` to calloc'd memory; a WGPUStringView
   argument of a callback is two integers (data, length). Every descriptor is calloc + set_field_* at the offsets of
   webgpu.h v29 (the constants on top of gpu.warp; any 64-bit machine). x86-64 SysV puts a struct over 16 bytes on
   the stack, which native_call cannot: there the callback infos would need a `struct` parameter kind.
3. Bulk copies: a `bytes` result (a C function's malloc'd {i64 length, bytes} read back as a text and freed, driver
   c_result) with lib/c.warp field_bytes / set_field_bytes; `f32s` and `i32s` parameters (a list of numbers as one C
   array of floats or int32s, its byte count the `length` after it). gpu.warp stages a buffer by
   `memcpy(into: handle, from: i32s, size: length)` (floats as their f32_bits) and reads one back by field_bytes.
4. The host words: the driver's paint (driver.warp painted_pixels) calls the module's host_shader_shown (1 shown in
   the window, 0 none open, -1 failed) and else host_shader_painted, which copies the RGBA rows into the driver's
   buffer (a handle id of it); a list of pixels goes to host_pixels_shown as RGBA bytes. gpu_compute is
   host_gpu_compute like any host word the driver doesn't answer. A GPU list's cells (extensions/gpu_lists) are in
   the program's memory: gpu_list_place and gpu_list_cell (library words) find them, the system value
   "program memory" (driver.warp, kept at each host word it hands the module) says where that memory starts.
5. The values: paint's values map is laid out in warp (values_laid_out, as host-gpu.js valuesLayout), its built-in
   holes (width, time, mouse …) filled from the module's own window state (value_of); no text round trip.
Gone with it: foreign.c's gpu_painted, values_filled, shader_shown, pixels_shown and the library-word mechanism
(HOST_WORD_LIBRARIES, warp_lazy_host_callback), lib/native/warp_node.h and warp_layout.h, scripts/node_layout.warp.
Checked headless: test/gpu (11), test_gpu_matches_cpu.sh, test_paint_sizes.sh, the samples that paint (gaussian splats
and shader trails byte-identical PNGs). Not checked here: a shader shown in a visible window (host_shader_shown).

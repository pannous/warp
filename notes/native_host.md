# The native host module in warp (decision window-in-warp; cards window-ffi-replace, warp-sound-c)

What the playground answers in JavaScript beyond the driver's own words, the native warp answers in warp:
lib/native_host.warp, one module of two parts, native_host/window.warp (SDL3's window, CoreText/FreeType text) and
native_host/sound.warp (SDL3's audio, AudioToolbox / libmpg123 decoding, CoreMIDI), over C libraries by plain FFI.
No C wrappers: crates/warp-window/warp_window.c and crates/warp-sound/warp_sound.c (miniaudio) are gone.

## How the driver reaches it
- modules/native_host.wasm is built by the rebuild (scripts/rebuild.sh) like every standard module (it is in
  rewrites.warp std_module_names); nothing compiles lib/ at run time (no-lib-reads). Its parts are files it uses by
  path (`use "native_host/window.warp"`), so they are one module, one link, sharing on_mac, quiet_run and cf_text.
- The driver links it lazily at the first question (core/native/driver.warp `lazily_linked`, the same path as unicode for
  canonical_text), so a program that never asks about the window or sound never loads it, SDL3 nor a decoder.
- Its exports are the hook: `host_owns(name)` / `host_value(name)` / `host_value_set(name, value)` for system values
  (window_open, pointer, keys, mouse, dark mode, window size while open), `host_window_layer(w, h)` opens the window
  and hands libwarp_gpu its CAMetalLayer, `host_run_ended()` holds a still picture open and waits till the sounds are
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

## warp_gpu.c next? (assessed 2026-11-07)
crates/warp-gpu/warp_gpu.c (800 lines over wgpu-native's webgpu.h) could follow, as a third part, once three
general FFI mechanisms exist; its WGSL text work (uniform layout, previous-frame detection) and the gpu_compute host
word's reading and building of warp lists (warp_node.h over wasmtime) become plain warp, simpler than in C:
1. Callbacks (card ffi-callbacks): done, lib/callbacks.warp `c_callback(f, ["handle", "i64"])` hands C one of 16
   foreign.c trampolines (six integer registers each), which calls the driver's c_callback_answered, which calls the
   module's export c_callback_fired(slot, …) during the C call; a "handle" argument arrives as a handle. Float
   arguments and more than 16 callbacks a run are refused; wgpu's callbacks take pointers and integers only.
2. Structs passed by value: wgpuInstanceRequestAdapter, wgpuBufferMapAsync … take a WGPU*CallbackInfo (5 pointers,
   40 bytes) by value; foreign.c's calls pass only registers of scalars. ARM64 passes such a struct by reference to a
   copy, x86-64 SysV on the stack: a `struct(bytes)` parameter kind in the driver's C call.
3. Bulk copies between C memory and warp lists: a frame's pixels (width × height × 4 bytes read back) and compute
   buffers both ways; one FFI call a value (as the sound decoder reads now) is ~a million calls a 640×480 frame. A
   `copied(memory, count, type): list` and its reverse in foreign.c would serve the decoder too.
Descriptors themselves (nested structs, WGPUStringView fields, chained structs) are calloc + set_field_* as far as
they are written into memory, verbose but fine.

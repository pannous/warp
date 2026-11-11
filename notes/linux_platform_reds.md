# Linux platform reds

The full suite runs on Linux now (the cloud routine, test/results.txt). Of its cases, 118 are ✗ there and were ✓ in
the last Mac run (`git show 7403f5eeda:test/results.txt`, 2026-11-09), matched by file and case text, never by line
(Linux run: 50b10f6, 2026-11-10; line numbers below are that run's). Each was rerun on a Linux x86_64 box
(card cloud-suite-failures, branch of the same name) on a fresh `warp rebuild pure` of main.

## Platform: the Mac has it, a Linux box does not (mark these)

### JavaScript through JavaScriptCore (33)
`use js …` and `use webview` run JavaScript in the system's web view, natively macOS's JavaScriptCore by the
Objective-C runtime (lib/host/native_host/webview.warp: `objc_autoreleasePoolPush from 'objc'`). Linux has no
libobjc and no system web view: "no C function objc_autoreleasePoolPush in its library". A Linux backend (WebKitGTK,
or an embedded JS engine) would be a card of its own; until then a refusal naming the platform would read better.
- test/ffi/test_foreign_js.pairs.warp:4, 5, 6, 7, 10, 14, 17
- test/ffi/test_foreign_member_assignment.pairs.warp:4, 5
- test/ffi/test_js_callbacks.pairs.warp:4
- test/ffi/test_web_idl.pairs.warp:4, 5, 6, 9, 12, 15, 16, 17, 20, 23, 24, 27, 30, 31, 32
- test/modules/test_js_globals.pairs.warp:2, 3, 4
- test/text/test_interpolation.pairs.warp:46, 47, 48
- test/web/test_webview.pairs.warp:3, 4

### GPU (16)
gpu_compute, gpu_render and a shader's paint run on wgpu (lib/host/native_host/gpu.warp, libwgpu_native.so beside
warp). The box has no GPU adapter. Instead of the refusal device_ready() means to give ("no GPU adapter: …"), the run
traps: "Out-of-bounds access in linear memory or a table" (a bug of its own, see the report: the `try … else 7` cases
would pass with a refusal).
- test/functions/test_host_word_overloads.pairs.warp:5
- test/gpu/test_gpu_compute.pairs.warp:2, 3, 5
- test/gpu/test_gpu_compute_in_place.pairs.warp:2, 3, 4, 5, 6
- test/gpu/test_gpu_f16.pairs.warp:2
- test/gpu/test_gpu_int_lists.pairs.warp:4
- test/gpu/test_gpu_render.pairs.warp:2, 3, 4
- test/programs/test_paint_shader.pairs.warp:3
- test/web/test_webgpu.pairs.warp:4

### raylib not installed (14)
The C library raylib is on the Mac (Homebrew), not on the box: "no C function GetTime in its library, or the library
not found". Installing libraylib on the cloud image turns these green.
- test/ffi/test_c_enums.pairs.warp:8, 9
- test/ffi/test_ffi_float_results.warp:4, 6, 8
- test/ffi/test_ffi_signatures.warp:27, 29, 31, 33, 35, 38, 40, 43, 45

### libc answers differently (2)
Both are C's own results, which the tests pin as macOS gives them: glibc's isdigit answers its ctype mask (2048), C
promises only nonzero; glibc's erf(1.0) rounds the last digit up (…149 against macOS's …148; the test's header names
macOS libm already). Questions for the supervisor below; the expectations are not changed.
- test/ffi/test_ffi_gaps.pairs.warp:12 (`use c; isdigit(55)` 1, got 2048)
- test/ffi/test_foreign_lines.warp:4 (`erf(1.0)` 0.8427007929497148, got …149)

### No display (1)
The box is headless: "display.width: this machine shows no display".
- test/programs/test_window_size.pairs.warp:4

### WAV decoding needs SDL3 (1)
The Mac decodes a WAV with AudioToolbox, Linux with SDL3 (lib/host/native_host/sound.warp), which the box lacks.
- test/web/test_rendered_samples.warp:4

## The cloud box, not the platform (fix the routine's setup)

### No network to warp.pannous.com (6)
The box's proxy refuses the site (curl: CONNECT tunnel failed, response 403).
- test/parser/test_scheme_relative_url.warp:4
- test/web/test_fetch_later.warp:4, 7
- test/web/test_fetch_signals.pairs.warp:1, 2, 3

### Packages not linked (8)
scripts/full-suite.sh links the registry's packages (scripts/link-packages.sh: a clone of pannous/warp-packages,
uniscript's C library built by cargo beside warp); the cloud run had none: "packages/colors/colors.warp: no such file",
"no C function uniscript_error in its library".
- test/modules/test_use_path_literal.pairs.warp:5
- test/modules/test_uniscript_lib.pairs.warp:2, 3, 4, 5
- test/modules/test_versions.pairs.warp:32
- test/web/test_uniscript.pairs.warp:4
- test/web/test_uniscript.warp:4

### Stale prebuilt module (5)
`log(x, base) is called with 1 argument at lib/random.warp`: green on a fresh build here; the cloud run's prebuilt
modules were older than its lib/.
- test/modules/test_random_distributions.pairs.warp:14, 15, 16, 20, 21

## Real (not platform)

Fixed on the branch (lib/ and a test fixture):
- lib/repl.warp: `use text` (remove_prefix, since f246a7a). test/modules/test_repl_show_value.pairs.warp:2–16
- lib/signals.warp: `on change` of a program's derived name (`total := …`) matched no rule since 7657401.
  test/control/test_event_footguns.warp:31, test/control/test_reactive_ports.warp:6, 33,
  test/control/test_variable_signals.warp:18
- lib/compute_toys.warp: `(?:` groups, which glibc's regcomp refuses (macOS REG_ENHANCED took them).
  test/modules/folder_beside/test_module_folder_beside.pairs.warp:3, test/modules/test_module_slices.pairs.warp:4
- test/control/test_try_host_failure.pairs.warp:7: `write` makes the folders it needs, so as root on Linux
  /no/such/folder was made; the fixture now writes under /dev/null, which fails everywhere.

Left (core causes, in the branch's report):
- test/programs/test_draw_label.pairs.warp:4, 5, test/programs/test_draw_label.warp:14: text_coverage(words, size)
  without `use draw` (c569529 gave the host line a font; lib/draw.warp has the two-value form again, which `use draw`
  programs reach, the bare call reaches the host line only)
- test/types/test_type_calls.pairs.warp:4, 5: `hex(123)` is lib/color.warp's hex(c: Color) by `n: int as Color`,
  not the type call
- test/programs/test_draw.warp:26: a blank canvas pixel 0 == black (3fadc35 made blanks 0; `0 == black` converts 0 to
  an opaque Color)
- test/control/test_meta_values.warp:8: a method's name shows its overload mark, "length·P"
- test/control/test_or_error_type.pairs.warp:13: `def f(x) as int? := x` refused ("int? is no type")
- test/types/test_conversion_phrase.pairs.warp:5: `Length as text := "L"` no longer reaches str(3m)
- test/types/test_typed_parse_flat_guards.pairs.warp:2: parse("{}", User) with an optional field now gives User{}
  instead of the refusal (an upgrade candidate)

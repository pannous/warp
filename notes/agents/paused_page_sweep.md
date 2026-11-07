# Paused page sweep (warp-playground, 2026-11-07 16:5x, base 160161ae8c): partial, stopped by the pause
Native: 247/263 (test/samples.txt). Page lines before the stop:
```
FAIL samples/apps/checksum_inventory.warp: read /warp/web/playground/warp-lib/modules/reals.wasm failed: No such file or directory
FAIL samples/apps/comment_board.warp: read /warp/web/playground/warp-lib/modules/html.wasm failed: No such file or directory
FAIL samples/apps/config_validator.warp: read /warp/web/playground/warp-lib/modules/json_tools.wasm failed: No such file or directory
FAIL samples/apps/deadlines.warp: read /warp/web/playground/warp-lib/modules/tables.wasm failed: No such file or directory
FAIL samples/apps/inventory_report.warp: read /warp/web/playground/warp-lib/modules/tables.wasm failed: No such file or directory
FAIL samples/apps/invoice_report.warp: read /warp/web/playground/warp-lib/modules/tables.wasm failed: No such file or directory
FAIL samples/apps/log_analyzer.warp: read /warp/web/playground/warp-lib/modules/tables.wasm failed: No such file or directory
FAIL samples/apps/mail_merge.warp: read /warp/web/playground/warp-lib/modules/tables.wasm failed: No such file or directory
FAIL samples/apps/name_matcher.warp: read /warp/web/playground/warp-lib/modules/tables.wasm failed: No such file or directory
FAIL samples/apps/open_hours.warp: read /warp/web/playground/warp-lib/modules/reals.wasm failed: No such file or directory
FAIL samples/apps/recipe_scaler.warp: read /warp/web/playground/warp-lib/modules/tables.wasm failed: No such file or directory
FAIL samples/apps/settings_cli.warp: read /warp/web/playground/warp-lib/modules/path.wasm failed: No such file or directory
FAIL samples/apps/site_generator.warp: read /warp/web/playground/warp-lib/modules/path.wasm failed: No such file or directory
FAIL samples/apps/todo_cli.warp: read /warp/web/playground/warp-lib/modules/path.wasm failed: No such file or directory
FAIL samples/apps/url_report.warp: read /warp/web/playground/warp-lib/modules/url.wasm failed: No such file or directory
FAIL samples/crates/regex_and_sha2.warp: read /warp/web/playground/warp-lib/modules/reals.wasm failed: No such file or directory
FAIL samples/golf/evil-numbers.warp: read /warp/web/playground/warp-lib/modules/slugs.wasm failed: No such file or directory
FAIL samples/golf/pernicious-numbers.warp: read /warp/web/playground/warp-lib/modules/slugs.wasm failed: No such file or directory
FAIL samples/golf/quine.warp: read /warp/web/playground/warp-lib/modules/slugs.wasm failed: No such file or directory
FAIL samples/gpu/gpu_visualizer.warp: stopped after 30 s: the program may not terminate
FAIL samples/gpu/julia.warp: stopped after 30 s: the program may not terminate
FAIL samples/gpu/manual_3d_object_viewer.warp: stopped after 30 s: the program may not terminate
FAIL samples/gpu/ocean.warp: stopped after 30 s: the program may not terminate
FAIL samples/gpu/sdf_scene.warp: stopped after 30 s: the program may not terminate
FAIL samples/gpu/voronoi.warp: stopped after 30 s: the program may not terminate
FAIL samples/gpu/warp_compiled_wgsl_shader.warp: stopped after 30 s: the program may not terminate
FAIL samples/graphics/paint_function.warp: stopped after 30 s: the program may not terminate
FAIL samples/graphics/shader_brush.warp: read /warp/web/playground/warp-lib/modules/html.wasm failed: No such file or directory
FAIL samples/modules/csv_report.warp: read /warp/web/playground/warp-lib/modules/tables.wasm failed: No such file or directory
FAIL samples/modules/packages.warp: packages/colors/colors.warp: no such file to use
FAIL samples/modules/rust_crates.warp: program.shown_from is not a function
FAIL samples/modules/slug_paths.warp: read /warp/web/playground/warp-lib/modules/path.wasm failed: No such file or directory
EXP  samples/native/c_calls.warp: page: "primitive mode cannot emit getenv from 'c' but in a native build: a primitive file's C calls run by `warp build native` at samples/native/c_calls.warp:6:1"
FAIL samples/servers/hosting.warp: program.shown_from is not a function
FAIL samples/servers/route_headers.warp: program.shown_from is not a function
FAIL samples/servers/server.warp: read /warp/web/playground/warp-lib/modules/reals.wasm failed: No such file or direct
```
Leads: many FAIL read /warp/web/playground/warp-lib/modules/<m>.wasm missing (build.sh without lagging does not build the modules?); program.shown_from is not a function (servers, rust_crates); gpu/paint_function 30 s timeouts.

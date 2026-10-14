mod test_accessibility;
#[cfg(all(feature = "native", target_os = "macos"))] // the system web view: macOS only for now
mod test_app_keys;
#[cfg(feature = "native")] // runs node
mod test_assistant_edits; // card put-editor: the chat's code applied as edits
#[cfg(feature = "native")] // runs wrangler dev and node
mod test_assistant_proxy; // card assistant-proxy
mod test_async_data; // the browser: a task Worker fetches into shared memory, read at the check points
mod test_attribute_values; // card attribute-values
mod test_bound_field_types; // card guest-type
#[cfg(feature = "native")] // runs node
mod test_browser_tables; // card orm-browser-stores
#[cfg(all(feature = "native", target_os = "macos"))] // the system web view: macOS only for now
mod test_built_app; // card build-links
#[cfg(feature = "native")] // builds a site natively
mod test_bundle_budget;
mod test_class_components;
#[cfg(feature = "native")] // runs node
mod test_completion_terse; // card completion-terse: the gray continuation is code only
#[cfg(feature = "native")] // runs node
mod test_completion_words; // cards code-completion-should, completion-must
#[cfg(feature = "native")] // runs the module natively
mod test_component_holes; // card web-fine-components
mod test_components;
mod test_css_transitions;
#[cfg(feature = "native")] // builds a site natively, opens it in headless Chrome (agent-browser)
mod test_database_site; // database[k] in a built site
#[cfg(feature = "native")] // wrangler dev runs the Worker (card cloud-deploy)
mod test_deploy;
#[cfg(feature = "native")] // runs node
mod test_dev_marks; // the dev overlay marks the failing word as the editor does
#[cfg(feature = "native")] // `warp dev` serves HTTP on a port
mod test_dev_server;
#[cfg(feature = "native")] // builds a site natively, opens it in headless Chrome (agent-browser)
mod test_dom_pages;
#[cfg(feature = "native")] // runs node
mod test_editor_mode; // card playground-editor
#[cfg(feature = "native")] // runs node
mod test_editor_shortcuts; // card keyboard-shortcuts
mod test_element_events;
#[cfg(feature = "native")] // the warning is the native run's
mod test_element_handler_quiet;
mod test_expert_guide;
mod test_field_events; // card click-key
mod test_form_bindings;
mod test_form_routes; // card g_mSEw
mod test_glued_attribute_content; // card words-glue
mod test_gpu_auto; // card gpu-auto
mod test_gpu_functions; // card gpu-functions
mod test_guide; // the language guide on the playground page
#[cfg(feature = "native")] // runs node
mod test_guide_heading_links; // card heading-links
mod test_guide_sections; // card guide-lists
mod test_headless_pages;
#[cfg(feature = "native")] // src/web/site.rs is native
mod test_host_parts; // the parts of host.js a site ships
#[cfg(feature = "native")] // src/web/site.rs is native
mod test_host_parts_std; // std_pure / std_io parts chosen by the std modules a program calls
mod test_html_escape; // card cleanup-html-escape
mod test_html_meta; // card html-meta
#[cfg(feature = "native")] // lib/markup.warp against src/html.rs, natively
mod test_html_render;
mod test_i18n;
mod test_keyed_lists;
#[cfg(feature = "native")] // runs python3
mod test_keyword_registry; // card where-infix
mod test_markup_holes;
mod test_markup_lines; // card todo-app
mod test_markup_mistakes; // card our-html
mod test_markup_nesting; // card markup-content
mod test_markup_renderer;
mod test_markup_scripts;
mod test_markup_tags;
#[cfg(feature = "native")] // a server on a port, HTTP requests
mod test_missing_row_404; // card missing-row-404
mod test_missing_use; // card clickable-hint
mod test_page_custom_events; // card page-custom-event
mod test_page_play_file; // card browser-play
mod test_page_rendered_sound; // card playground-refuses
#[cfg(feature = "native")] // builds a site natively
mod test_page_server_data; // a main-level statement changing server data stays on the server
mod test_page_tests;
#[cfg(feature = "native")] // runs node
mod test_playground_paint; // card canvas-zero
mod test_primer; // card language-primer: the assistant's system prompt, /llms.txt
mod test_rendered_at; // card sample-accurate
mod test_rendered_fades; // card sound-gain-ramps
mod test_rendered_pan; // card stereo-pan
mod test_rendered_samples; // card sound-samples
mod test_rendered_task_sounds; // card playground-drops
mod test_rendering_itself; // card playground-render
#[cfg(feature = "native")] // a server on a port, HTTP requests
mod test_request_fields; // card g_mQ9U
#[cfg(feature = "native")] // a site build and a server on a port
mod test_route_data; // P221
#[cfg(feature = "native")] // a server on a port, HTTP requests
mod test_route_error_positions; // card runtime-errors
#[cfg(feature = "native")] // a server on a port, HTTP requests
mod test_route_header; // card web-route-header
#[cfg(feature = "native")] // runs wasm-split
mod test_route_helpers; // card web-split-helpers
#[cfg(feature = "native")] // builds a site (src/web/site.rs)
mod test_route_modules;
#[cfg(feature = "native")] // a server on a port and its SQLite table
mod test_route_phrase_body; // card first-first
#[cfg(feature = "native")] // the page path of a native render (host::with_page_path)
mod test_routes;
mod test_safari_imports;
#[cfg(feature = "native")] // runs the warp binary
mod test_sandbox_refusals; // card sandbox-exec
mod test_sandboxed_programs; // card ferron-hosting
mod test_script_words; // card browser-twin
mod test_select_options; // card select-shorthand
#[cfg(feature = "native")] // a server on a port, HTTP requests
mod test_serve_error_body; // card serve-error-body
#[cfg(feature = "native")] // a server on a port, HTTP requests
mod test_serve_opens_page; // card samples-serving
#[cfg(feature = "native")] // a server on a port, HTTP requests and its SQLite tables
mod test_served_forms; // card todo-app
#[cfg(feature = "native")] // a server on a port and its SQLite tables
mod test_served_tables; // card sample-server
#[cfg(feature = "native")] // a site build with the native compiler
mod test_server_functions_in_page; // card route-sample
#[cfg(feature = "native")] // a server on a port, HTTP requests
mod test_server_route_paths; // cards server-path, route-star
#[cfg(feature = "native")] // a site build with the native compiler
mod test_server_rpc_everywhere; // card rpc-everywhere
#[cfg(feature = "native")] // a site build with the native compiler
mod test_server_rpc_stub; // card rpc-stub
mod test_signal_sound; // card real-time
#[cfg(feature = "native")] // warp build --site writes files with the native compiler
mod test_site;
#[cfg(feature = "native")] // builds a site natively
mod test_site_animation; // card site-build
#[cfg(feature = "native")] // builds a site natively
mod test_site_failure; // the dev overlay's message
#[cfg(all(feature = "native", target_os = "macos"))] // the system web view
mod test_site_holes;
#[cfg(feature = "native")] // builds a site natively
mod test_site_tasks; // card site-tasks
#[cfg(feature = "native")] // runs node
mod test_sleep_run_limit; // card sleep-run-limit
#[cfg(feature = "native")] // a server on a port, HTTP requests
mod test_status_reason; // card status-reason
mod test_style_rules;
mod test_styles;
mod test_tag_lists;
mod test_task_files; // card task-sound-play
mod test_task_voices; // card playground-go
mod test_text_holes; // card web-fine-text-nodes
mod test_transitions;
mod test_typed_variable_not_markup; // card annotation-html
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_uniscript;
#[cfg(feature = "native")] // runs the WAGI module with wasmtime and WASI
mod test_wagi; // card fermyon-hosting
#[cfg(feature = "native")] // runs the WAGI module with wasmtime and WASI
mod test_wagi_headers; // card wagi-build
#[cfg(feature = "native")] // a server on a port, HTTP requests
mod test_warp_serve; // P222
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_web;
mod test_web_apis;
mod test_web_playground;
#[cfg(feature = "native")] // a server on a port, HTTP requests
mod test_web_server;
mod test_webgpu;
mod test_webgpu_ints;
mod test_webgpu_named; // card web-apis-rest
#[cfg(all(feature = "native", target_os = "macos"))] // the system web view: macOS only for now
mod test_webview; // card read-chron // card key-app // card web-fine-rest

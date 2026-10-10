#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_uniscript;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_web;
mod test_markup_tags;
mod test_markup_scripts;
mod test_web_playground;
#[cfg(feature = "native")] // runs node
mod test_editor_shortcuts; // card keyboard-shortcuts
#[cfg(feature = "native")] // runs node
mod test_editor_mode; // card playground-editor
mod test_missing_use; // card clickable-hint
mod test_guide; // the language guide on the playground page
mod test_expert_guide;
mod test_primer; // card language-primer: the assistant's system prompt, /llms.txt
#[cfg(feature = "native")] // a server on a port, HTTP requests
mod test_web_server;
#[cfg(feature = "native")] // `warp dev` serves HTTP on a port
mod test_dev_server;
mod test_async_data; // the browser: a task Worker fetches into shared memory, read at the check points
#[cfg(feature = "native")] // warp build --site writes files with the native compiler
mod test_site;
#[cfg(feature = "native")] // builds a site (src/site.rs)
mod test_route_modules;
#[cfg(feature = "native")] // builds a site natively
mod test_bundle_budget;
#[cfg(feature = "native")] // src/site.rs is native
mod test_host_parts; // the parts of host.js a site ships
#[cfg(feature = "native")] // builds a site natively
mod test_site_tasks; // card site-tasks
mod test_rendering_itself; // card playground-render
#[cfg(feature = "native")] // lib/markup.warp against src/html.rs, natively
mod test_html_render;
mod test_markup_renderer;
#[cfg(feature = "native")] // the page path of a native render (host::with_page_path)
mod test_routes;
mod test_element_events;
mod test_class_components;
mod test_components;
mod test_keyed_lists;
mod test_markup_holes;
mod test_form_bindings;
mod test_styles;
mod test_style_rules;
mod test_headless_pages;
#[cfg(feature = "native")] // builds a site natively, opens it in headless Chrome (agent-browser)
mod test_dom_pages;
#[cfg(feature = "native")] // wrangler dev runs the Worker (card cloud-deploy)
mod test_deploy;
mod test_page_tests;
mod test_transitions;
mod test_css_transitions;
mod test_web_apis;
mod test_accessibility;
mod test_i18n;
mod test_webgpu;
mod test_webgpu_ints;
mod test_gpu_auto; // card gpu-auto
mod test_tag_lists;
mod test_safari_imports;
#[cfg(feature = "native")] // a site build with the native compiler
mod test_server_functions_in_page; // card route-sample
#[cfg(feature = "native")] // a site build with the native compiler
mod test_server_rpc_stub; // card rpc-stub
#[cfg(feature = "native")] // a site build with the native compiler
mod test_server_rpc_everywhere; // card rpc-everywhere
#[cfg(feature = "native")] // a server on a port, HTTP requests
mod test_warp_serve; // P222
#[cfg(feature = "native")] // a site build and a server on a port
mod test_route_data; // P221
#[cfg(feature = "native")] // a server on a port and its SQLite tables
mod test_served_tables; // card sample-server
#[cfg(feature = "native")] // a server on a port, HTTP requests and its SQLite tables
mod test_served_forms; // card todo-app
#[cfg(feature = "native")] // a server on a port, HTTP requests
mod test_request_fields; // card g_mQ9U
mod test_markup_lines; // card todo-app
mod test_form_routes; // card g_mSEw
mod test_guide_sections; // card guide-lists
mod test_typed_variable_not_markup; // card annotation-html
#[cfg(feature = "native")] // a server on a port, HTTP requests
mod test_serve_error_body; // card serve-error-body
#[cfg(feature = "native")] // a server on a port, HTTP requests
mod test_missing_row_404; // card missing-row-404
#[cfg(feature = "native")] // a server on a port, HTTP requests
mod test_server_route_paths; // cards server-path, route-star
#[cfg(feature = "native")] // a server on a port and its SQLite table
mod test_route_phrase_body; // card first-first
mod test_sandboxed_programs; // card ferron-hosting
mod test_page_rendered_sound; // card playground-refuses
mod test_page_play_file; // card browser-play

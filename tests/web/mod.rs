#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_uniscript;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_web;
mod test_markup_tags;
mod test_web_playground;
mod test_missing_use; // card clickable-hint
mod test_guide; // the language guide on the playground page
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
#[cfg(feature = "native")] // lib/markup.wasp against src/html.rs, natively
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
mod test_page_tests;
mod test_transitions;
mod test_css_transitions;
mod test_web_apis;
mod test_accessibility;
mod test_i18n;
mod test_webgpu;

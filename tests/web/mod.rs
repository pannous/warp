#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_uniscript;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_web;
mod test_markup_tags;
mod test_web_playground;
#[cfg(feature = "native")] // a server on a port, HTTP requests
mod test_web_server;
mod test_async_data; // the browser: a task Worker fetches into shared memory, read at the check points
#[cfg(feature = "native")] // warp build --site writes files with the native compiler
mod test_site;
#[cfg(feature = "native")] // builds a site natively
mod test_bundle_budget;
mod test_html_render;
mod test_markup_renderer;
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

#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_uniscript;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_web;
mod test_markup_tags;
mod test_web_playground;
#[cfg(feature = "native")] // a server on a port, HTTP requests
mod test_web_server;
#[cfg(feature = "native")] // warp build --site writes files with the native compiler
mod test_site;
mod test_html_render;
mod test_element_events;
mod test_components;
mod test_keyed_lists;
mod test_form_bindings;
mod test_styles;

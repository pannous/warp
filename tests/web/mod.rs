#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_uniscript;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_web;
mod test_markup_tags;
mod test_web_playground;
#[cfg(feature = "native")] // a server on a port, HTTP requests
mod test_web_server;
mod test_html_render;
mod test_element_events;
mod test_class_components;
mod test_components;
mod test_keyed_lists;

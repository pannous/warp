#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_uniscript;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_web;
mod test_markup_tags;
mod test_web_playground;
mod test_html_render;

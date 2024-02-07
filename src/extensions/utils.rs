use std::io::ErrorKind;
//noinspection ALL
#[cfg(not(feature = "wasm"))]
use reqwest::blocking::{Client, get, Response};

#[cfg(feature = "wasm")]
extern { fn download(url: &str) -> String; } // String not FFI-safe

#[must_use]
#[cfg(not(feature = "wasm"))]
pub fn download(url: &str) -> String {
    let empty:String = String::from("");

    let response = match get(url) {
        Ok(res) => res,
        Err(_) => return empty
    };
    return match response.bytes() {
        Ok(bytes) => {
            let s = String::from_utf8(bytes.to_vec());
            match s {
                Ok(s) => s,
                Err(_) => empty
            }
        },
        Err(_) => empty
    }
}
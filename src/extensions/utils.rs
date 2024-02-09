use std::io::ErrorKind;
//noinspection ALL
#[cfg(not(feature = "wasm"))]
#[cfg(not(test))]
use reqwest::blocking::{Client, get, Response};
use crate::extensions::strings::StringExtensions;


// #[cfg(any(feature = "wasm",test))]
#[cfg(feature = "wasm")]
extern { fn download(url: &str) -> String; } // String not FFI-safe

#[cfg(test)]
fn download(url: &str) -> String { String::from("mock".s() + url) }


#[must_use]
#[cfg(not(feature = "wasm"))]
#[cfg(not(test))]
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


pub trait FileExtensions {
    // std::fs::File does not directly expose the file name.
    fn name(&self) -> String;
    fn path(&self) -> String;
}

impl FileExtensions for std::fs::File {
    fn name(&self) -> String {
        "std::fs::File does not expose the file name. ".to_string()
    }

    fn path(&self) -> String {
        "std::fs::File does not expose the file name or path.".to_string()
    }

}

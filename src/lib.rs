// shared code with wasp tests etc
// only lib.rs allows reexporting as:
// use wasp::extensions::*; etc
pub mod extensions;// reexported for tests

// use crate::extensions::*; // crate for F12
pub use extensions::numbers::*;
pub use extensions::strings::*;
pub use extensions::lists::*;
pub use extensions::utils::*;

pub mod parser;

pub fn init_lib(){
    println!("init lib")
}
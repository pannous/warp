// the library names itself by @rpath (macOS) or its soname (Linux), so whoever links it finds it on its own rpath:
// beside warp or beside the executable
fn main() {
	match std::env::var("CARGO_CFG_TARGET_OS").as_deref() {
		Ok("macos") => println!("cargo:rustc-cdylib-link-arg=-Wl,-install_name,@rpath/libwarp_wasmtime.dylib"),
		Ok("linux") => println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,libwarp_wasmtime.so"),
		_ => {}
	}
}

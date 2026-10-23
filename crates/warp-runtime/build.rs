//! warp-host's compilers beside it (card warp-host-compiler-beside): a build with the feature `host` copies
//! compiler-self.wasm and compiler.wasm next to the binary (target/<profile>/warp-host), where warp-host looks first.
//! They come from WARP_HOST_COMPILERS (a folder), else the checkout's data/ (`warp rebuild` writes them there).
use std::path::{Path, PathBuf};

const COMPILERS: [&str; 2] = ["compiler-self.wasm", "compiler.wasm"];

fn main() {
	println!("cargo:rerun-if-env-changed=WARP_HOST_COMPILERS");
	if std::env::var_os("CARGO_FEATURE_HOST").is_none() {
		return;
	}
	let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
	let source = match std::env::var_os("WARP_HOST_COMPILERS") {
		Some(folder) => PathBuf::from(folder),
		None => manifest.join("../../data"),
	};
	// OUT_DIR is <target>/<profile>/build/warp-runtime-<hash>/out: the binaries are in <target>/<profile>
	let out = PathBuf::from(std::env::var("OUT_DIR").unwrap_or_default());
	let Some(profile) = out.ancestors().nth(3).map(Path::to_path_buf) else { return };
	for name in COMPILERS {
		let from = source.join(name);
		println!("cargo:rerun-if-changed={}", from.display());
		if !from.is_file() {
			println!("cargo:warning=no {} to put beside warp-host: `warp rebuild` writes it to data/, or set WARP_HOST_COMPILERS", from.display());
			continue;
		}
		let to = profile.join(name);
		// a copy removed from target/ is put back by the next build
		println!("cargo:rerun-if-changed={}", to.display());
		// a new file moved into place: a running warp-host may be reading the old one
		let part = profile.join(format!("{name}.part"));
		if let Err(problem) = std::fs::copy(&from, &part).and_then(|_| std::fs::rename(&part, &to)) {
			println!("cargo:warning=cannot copy {} to {}: {problem}", from.display(), to.display());
		}
	}
}

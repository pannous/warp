//! warp-host's compilers beside it (card warp-host-compiler-beside): a build with the feature `host` copies
//! warp-self.wasm and warp.wasm next to the binary (target/<profile>/warp-host), where warp-host looks first.
//! They come from WARP_HOST_COMPILERS (a folder), else the checkout's data/ (`warp rebuild` writes them there); a file of
//! the old name (compiler-self.wasm, compiler.wasm) there is copied under the new one (transition).
use std::path::{Path, PathBuf};

/// Each compiler's name, and its old name read when the new one is missing
const COMPILERS: [(&str, &str); 2] = [("warp-self.wasm", "compiler-self.wasm"), ("warp.wasm", "compiler.wasm")];

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
	for (name, legacy) in COMPILERS {
		let from = source.join(name);
		println!("cargo:rerun-if-changed={}", from.display());
		let from = if from.is_file() { from } else { source.join(legacy) };
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

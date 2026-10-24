// The compiler inputs hash (card compiler-cache-keys, notes/compiler_rebuilds.md): what a compiler build is judged
// current by, and the shared compiler store's key; scripts/compiler-inputs.sh computes the same 16 hex digits
use warp::compiler::self_hosted::compiler_inputs_hash;

#[test]
fn compiler_inputs_hash_is_16_hex_digits_and_stable() {
	let hash = compiler_inputs_hash();
	assert_eq!(hash.len(), 16, "{hash}");
	assert!(hash.chars().all(|c| c.is_ascii_hexdigit()), "{hash}");
	assert_eq!(hash, compiler_inputs_hash());
}

#[test]
fn compiler_inputs_hash_is_the_scripts() {
	let checkout = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
	let script = checkout.join("scripts/compiler-inputs.sh");
	if !script.exists() {
		return; // the script comes with card compiler-rebuild-policy
	}
	let lib = warp::modules::library_folder();
	let said = std::process::Command::new("bash").arg(&script).arg(&lib).arg(env!("WARP_SOURCE")).output().unwrap();
	assert_eq!(String::from_utf8_lossy(&said.stdout).trim(), compiler_inputs_hash());
}

// Card stub-race: the executable tests build from this checkout's own warp-runtime stub, never the shared
// target/debug/warp-runtime that another worktree's build replaces mid-run (12 empty outputs in batch 28's gate)

#[test]
fn test_runtime_stub_is_this_checkouts() {
	let stub = crate::common::runtime_stub();
	assert!(!stub.starts_with(std::path::Path::new(env!("CARGO_BIN_EXE_warp")).parent().unwrap()), "{}", stub.display());
	let run = std::process::Command::new(stub).output().unwrap();
	let said = String::from_utf8_lossy(&run.stderr);
	let checkout = concat!(env!("CARGO_MANIFEST_DIR"), "/crates/warp-runtime");
	assert!(said.contains(&format!("({} from {checkout})", env!("CARGO_PKG_VERSION"))), "{said}");
}

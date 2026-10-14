//! `warp --sandbox` (cards sandbox-exec, sandbox-hosted; notes/hosting.md): a sandboxed program starts no process and
//! writes files only inside its state folder; it reaches the network. The capability check in pipeline.rs sees only
//! the words a program calls directly (`exec sh"…"` is os.exec through std_io, a Host word), so the places that start
//! a process or write a file ask here themselves, and refuse loudly.
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

const FLAG: &str = "warp --sandbox";
/// systemd's StateDirectory= (web/hosting/server/warp-lambda@.service); without it the working directory
const STATE_FOLDER_VARIABLE: &str = "STATE_DIRECTORY";

/// Set once by `warp --sandbox` for its whole run: the threads tasks run on are sandboxed too
static PROCESS_SANDBOXED: AtomicBool = AtomicBool::new(false);

/// Every thread of this process sandboxed, for good
pub fn confine_process() {
	PROCESS_SANDBOXED.store(true, Ordering::SeqCst);
}

pub fn is_sandboxed() -> bool {
	PROCESS_SANDBOXED.load(Ordering::SeqCst) || crate::compiler::compile_context::read(|context| context.sandboxed)
}

/// The loud refusal of `word`, which needs `capability`
pub fn denial(word: &str, capability: &str) -> String {
	format!("capability denied: {word} needs the {capability} capability, which {FLAG} does not grant")
}

/// The refusal of `word` starting a process (os.exec, a sound player, the MIDI sender) when sandboxed
pub fn check_process(word: &str) -> Result<(), String> {
	match is_sandboxed() {
		true => Err(denial(word, "process")),
		false => Ok(()),
	}
}

/// The refusal of writing `path` when sandboxed and the path leaves the state folder: through `..`, a symbolic link
/// or an outside folder
pub fn check_write(path: &str) -> Result<(), String> {
	if !is_sandboxed() {
		return Ok(());
	}
	let folder = state_folder()?;
	let refusal = |why: &str| {
		Err(format!(
			"capability denied: writing {path} {why}: {FLAG} grants the file-write capability only inside the state folder {}",
			folder.display()
		))
	};
	let written = Path::new(path);
	if written.components().any(|part| part == Component::ParentDir) {
		return refusal("through ..");
	}
	let here = std::env::current_dir().map_err(|problem| problem.to_string())?;
	let absolute = here.join(written);
	if absolute
		.ancestors()
		.take_while(|ancestor| ancestor.starts_with(&here) && *ancestor != here)
		.any(|ancestor| ancestor.symlink_metadata().is_ok_and(|kept| kept.is_symlink()))
	{
		return refusal("through a symbolic link");
	}
	let existing = absolute.ancestors().find(|ancestor| ancestor.exists()).unwrap_or(&absolute);
	let missing = absolute.strip_prefix(existing).unwrap_or(Path::new(""));
	let resolved = existing.canonicalize().map_err(|problem| problem.to_string())?.join(missing);
	match resolved.starts_with(&folder) {
		true => Ok(()),
		false => refusal("outside it"),
	}
}

/// The folder a sandboxed program writes in, resolved
fn state_folder() -> Result<PathBuf, String> {
	let folder = std::env::var_os(STATE_FOLDER_VARIABLE).map_or_else(std::env::current_dir, |folder| Ok(folder.into()));
	folder.and_then(|folder| folder.canonicalize()).map_err(|problem| format!("no state folder: {problem}"))
}

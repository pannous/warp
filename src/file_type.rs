//! `warp register` (macOS, card g_gHmE): Finder and `open` know .warp files. A small applet in ~/Applications declares
//! the file type (conforming to source code, so any text editor is offered under Open With) and runs an opened file
//! with this warp in Terminal

use std::path::{Path, PathBuf};
use std::process::Command;

const APP_NAME: &str = "Warp Lang.app";
const BUNDLE_ID: &str = "com.pannous.warp-lang";
const TYPE_ID: &str = "com.pannous.warp.source";
const TYPE_DESCRIPTION: &str = "Warp source";
const EXTENSIONS: [&str; 2] = ["warp", "wasp"];
const CONFORMS_TO: [&str; 2] = ["public.source-code", "public.plain-text"];
const WARP_NAME: &str = "warp";
const LSREGISTER: &str = "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister";

/// Builds and registers the applet; the path of the app
pub fn register() -> Result<PathBuf, String> {
	let home = std::env::var("HOME").map_err(|_| "no HOME to put the app in".to_string())?;
	let app = Path::new(&home).join("Applications").join(APP_NAME);
	let warp = warp_command()?;
	std::fs::create_dir_all(app.parent().expect("in Applications")).map_err(|error| error.to_string())?;
	run("osacompile", &["-o", &app.to_string_lossy(), "-e", &applet_script(&warp)])?;
	let plist = app.join("Contents/Info.plist").to_string_lossy().to_string();
	for (key, value) in info_entries() {
		// replaced when there (a second registration), inserted otherwise
		let _ = run("plutil", &["-remove", key, &plist]);
		run("plutil", &["-insert", key, "-json", &value, &plist])?;
	}
	// signed again (ad hoc) after the edit, else Launch Services treats the declared type as untrusted
	run("codesign", &["--force", "--sign", "-", &app.to_string_lossy()])?;
	run(LSREGISTER, &["-f", &app.to_string_lossy()])?;
	Ok(app)
}

/// `warp` when the shell finds it (a Homebrew upgrade keeps working), else this binary's path
fn warp_command() -> Result<PathBuf, String> {
	let on_path = std::env::var_os("PATH").is_some_and(|path| std::env::split_paths(&path).any(|folder| folder.join(WARP_NAME).is_file()));
	match on_path {
		true => Ok(PathBuf::from(WARP_NAME)),
		false => std::env::current_exe().map_err(|error| format!("where is this warp? {error}")),
	}
}

/// Each opened file runs in a Terminal window of its own, which stays open with the program's output
fn applet_script(warp: &Path) -> String {
	let warp = warp.to_string_lossy().replace('\'', "'\\''");
	format!("on open theFiles\n\trepeat with theFile in theFiles\n\t\ttell application \"Terminal\" to do script \"'{warp}' \" & quoted form of POSIX path of theFile\n\tend repeat\n\ttell application \"Terminal\" to activate\nend open")
}

fn info_entries() -> [(&'static str, String); 3] {
	let texts = |items: &[&str]| format!("[{}]", items.iter().map(|item| format!("{item:?}")).collect::<Vec<_>>().join(","));
	let document_type = format!("[{{\"CFBundleTypeName\":{TYPE_DESCRIPTION:?},\"CFBundleTypeRole\":\"Viewer\",\"LSHandlerRank\":\"Owner\",\"LSItemContentTypes\":[{TYPE_ID:?}]}}]");
	let exported_type = format!("[{{\"UTTypeIdentifier\":{TYPE_ID:?},\"UTTypeDescription\":{TYPE_DESCRIPTION:?},\"UTTypeConformsTo\":{},\"UTTypeTagSpecification\":{{\"public.filename-extension\":{}}}}}]",
		texts(&CONFORMS_TO), texts(&EXTENSIONS));
	[("CFBundleIdentifier", format!("{BUNDLE_ID:?}")), ("CFBundleDocumentTypes", document_type), ("UTExportedTypeDeclarations", exported_type)]
}

fn run(program: &str, arguments: &[&str]) -> Result<(), String> {
	let output = Command::new(program).args(arguments).output().map_err(|error| format!("{program}: {error}"))?;
	match output.status.success() {
		true => Ok(()),
		false => Err(format!("{program} {}: {}", arguments.first().unwrap_or(&""), String::from_utf8_lossy(&output.stderr).trim())),
	}
}

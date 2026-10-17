//! card map-read: `x.size` of a map only the run time knows reads its entry size when it has one, as a literal map's
//! `{size: 5}.size` does; any other value (a list, a text, a map without the entry, a Path) still answers the count or
//! its own size word
use crate::is;

#[test]
fn a_size_entry_of_a_map_read_at_run_time() {
	is!("f(m) := m.size\nf({size: 9, a: 1, b: 2})", 9);
	is!("f(m) := m.size\nf({a: 1, b: 2})", 2);
	is!("f(m) := m.size\nf([1 2 3])", 3);
	is!("f(t) := t.size\nf(\"héllo\")", 5);
	is!("f(m) := m.length\nf({length: 4})", 4);
	is!("f(xs) := { s = 0; for i in 0 ..< xs.length { s += xs#(i+1) }; s }\nf([1 2 3])", 6);
}

#[cfg(feature = "native")] // writes a file: the browser's programs see no files of the test's
#[test]
fn the_size_entry_of_shell_info() {
	let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scratch/map_read");
	std::fs::create_dir_all(&folder).expect("scratch folder");
	let file = folder.join("six.txt");
	std::fs::write(&file, "hello\n").expect("a file of 6 bytes");
	let path = file.display();
	is!(&format!("use shell\ni = info \"{path}\"\ni.size"), 6);
	is!(&format!("use shell\np = Path(name: \"{path}\")\np.size"), 6);
}

/// reviewer: every uncalled word on a value that may be a map reads its entry first, not only counting words
#[cfg(feature = "native")] // writes a file, as above
#[test]
fn any_entry_of_shell_info() {
	let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scratch/map_read");
	std::fs::create_dir_all(&folder).expect("scratch folder");
	let file = folder.join("kind.txt");
	std::fs::write(&file, "hello\n").expect("a file");
	is!(&format!("use shell\ni = info \"{}\"\ni.kind", file.display()), "file");
}

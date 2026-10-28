//! The standard modules as the harness keeps them on `pure` (card rust-bootstrap): the embedded lib/<module>.warp
//! sources, their names, and the names each defines, read from the text (`warp help`)
/// The standard library's modules written in warp (notes/stdlib.md), embedded so `use list` needs no files
pub(crate) const STD_MODULES: [(&str, &str); 36] = [
	(PRELUDE_MODULE, include_str!("../../lib/prelude.warp")),
	("memory", include_str!("../../lib/memory.warp")),
	("net", include_str!("../../lib/net.warp")),
	("collections", include_str!("../../lib/collections.warp")),
	("hash", include_str!("../../lib/hash.warp")),
	("regex", include_str!("../../lib/regex.warp")),
	("file", include_str!("../../core/library/file.warp")),
	("json", include_str!("../../lib/json.warp")),
	("os", include_str!("../../core/library/os.warp")),
	("shell", include_str!("../../lib/shell.warp")),
	("list", include_str!("../../lib/list.warp")),
	("math", include_str!("../../lib/math.warp")),
	("fast_math", include_str!("../../lib/fast_math.warp")),
	("text", include_str!("../../lib/text.warp")),
	("random", include_str!("../../lib/random.warp")),
	("map", include_str!("../../lib/map.warp")),
	("time", include_str!("../../lib/time.warp")),
	("matrix", include_str!("../../lib/matrix.warp")),
	("draw", include_str!("../../lib/draw.warp")),
	("compute_toys", include_str!("../../lib/compute_toys.warp")),
	("markup", include_str!("../../lib/markup.warp")),
	("router", include_str!("../../lib/router.warp")),
	("wagi", include_str!("../../lib/wagi.warp")),
	("i18n", include_str!("../../lib/i18n.warp")),
	("units", include_str!("../../lib/units.warp")),
	("agent", include_str!("../../lib/agent.warp")),
	("sound", include_str!("../../lib/sound.warp")),
	("R", include_str!("../../lib/R.warp")),
	("PHP", include_str!("../../lib/PHP.warp")),
	("channels", include_str!("../../lib/channels.warp")),
	("extrema", include_str!("../../lib/extrema.warp")),
	("english", include_str!("../../lib/english.warp")),
	("case_table", include_str!("../../lib/case_table.warp")),
	("case_tables", include_str!("../../core/library/case_tables.warp")),
	("uncertain", include_str!("../../lib/uncertain.warp")),
	("wasmtime", include_str!("../../lib/wasmtime.warp")),
];
/// Other names of standard modules (word choices are aliases): `use graphics` is `use draw`
pub(crate) const STD_MODULE_ALIASES: [(&str, &str); 1] = [("graphics", "draw")];
/// The standard modules' folder (P194: std/ merged into lib/), embedded in the binary
pub(crate) const STD_FOLDER: &str = "lib";
/// lib/prelude.warp: the warp-written words of every program, loaded when mentioned (Loader::use_prelude)
pub(crate) const PRELUDE_MODULE: &str = "prelude";
/// The source of the standard module `name` (lib/<name>.warp, embedded)
pub fn std_module_source(name: &str) -> Option<&'static str> {
	std_module(name)
}

/// The standard modules' names: `list`, `math`, …
pub fn std_module_names() -> impl Iterator<Item = &'static str> {
	STD_MODULES.iter().map(|(module, _)| *module)
}

/// A standard module's source as the binary embeds it
pub fn std_module(name: &str) -> Option<&'static str> {
	let name = std_module_name(name);
	STD_MODULES.iter().find(|(module, _)| *module == name).map(|(_, source)| *source)
}

/// A standard module's own name for one of its aliases (`graphics` → draw)
fn std_module_name(name: &str) -> &str {
	STD_MODULE_ALIASES.iter().find(|(alias, _)| *alias == name).map_or(name, |(_, module)| *module)
}

/// The names a standard module defines at its top level, in their order (`ls(folder) := …`, `pi = 3.14…`), read from
/// its text; None for no standard module
pub fn std_module_definitions(module: &str) -> Option<Vec<String>> {
	let source = std_module(module)?;
	let defined = |line: &str| {
		let name: String = line.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '!').collect();
		let rest = line[name.len()..].trim_start();
		let defines = rest.starts_with('(') && line.contains(":=") || rest.starts_with(":=") || rest.starts_with("= ");
		(!name.is_empty() && defines).then_some(name)
	};
	let mut names: Vec<String> = vec![];
	for name in source.lines().filter_map(defined) {
		if !names.contains(&name) {
			names.push(name);
		}
	}
	Some(names)
}

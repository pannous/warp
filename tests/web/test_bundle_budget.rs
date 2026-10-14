// card web-bundle: what a hello-world site sends, gzipped, stays within its budget; the test prints each file so the
// suite's log tracks the sizes (notes/web_framework.md step 17). Svelte's hello world is about 3 KB, Solid's about 5 KB.
use std::io::Write;

const HELLO: &str = "p{ \"hello world\" }";
const FOLDER: &str = "scratch/bundle_budget";
/// 2026-10-07: 46.2 KB (host.js 23.8, app.wasm 17.5, markup.js 2.4, site.js 1.1, reader.js 1.1, index.html 0.2);
/// 40.5 KB with the case table as ranges (app.wasm 11.9); 32.1 KB with compacted scripts (host.js 16.7); 21.9 KB with
/// host.js split into parts a site ships only when its module imports them (host.js 6.1); 2026-10-12: 23.0 KB (app.wasm
/// 12.0, host.js 6.9), budget raised by 2 KB so it stops blocking branches while card web-bundle-growth wins it back;
/// 22.1 KB with paint, text_coverage and sound_samples in host-media.js (host.js 5.9), the budget a ratchet: today + 1 KB;
/// 20.9 KB with html_kebab ASCII-only, no Unicode case table in app.wasm (10.8); 19.8 KB with exact numbers (exact.js) and
/// std calls (std.js) only in the parts that carry them, plus markup's trimming (host.js 5.2); user, 2026-10-12: the
/// budget is 100 KB, plenty next to other sites, so it never blocks a branch again; the size is still printed
const BUDGET_GZIPPED_BYTES: usize = 100_000;

fn gzipped_size(bytes: &[u8]) -> usize {
	let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
	encoder.write_all(bytes).expect("gzip");
	encoder.finish().expect("gzip").len()
}

#[test]
fn a_hello_world_site_stays_within_its_budget() {
	let site = warp::web::site::build(HELLO, "hello", std::path::Path::new(FOLDER)).expect("the site");
	let sizes: Vec<(String, usize)> = site
		.files
		.iter()
		.map(|name| (name.clone(), gzipped_size(&std::fs::read(site.directory.join(name)).expect("a file"))))
		.collect();
	let total: usize = sizes.iter().map(|(_, size)| size).sum();
	sizes.iter().for_each(|(name, size)| println!("hello-world site: {name:<12} {size:>7} bytes gzipped"));
	println!("hello-world site: total        {total:>7} bytes gzipped (budget {BUDGET_GZIPPED_BYTES})");
	assert!(
		total <= BUDGET_GZIPPED_BYTES,
		"a hello-world site is {total} bytes gzipped, over its budget of {BUDGET_GZIPPED_BYTES}"
	);
}

// a site's scripts go without their comment lines and indentation; a template literal's lines stay as written
#[test]
fn a_site_script_is_compacted() {
	let script = "// what it does\nfunction f() {\n\t// a comment\n\treturn `a\n  // kept\n  b`;\n}\n\n";
	assert_eq!(warp::web::site::compacted(script), "function f() {\nreturn `a\n  // kept\n  b`;\n}\n");
}

// a comment after a statement goes too; a template literal's line keeps it
#[test]
fn a_site_script_loses_its_trailing_comments() {
	let script = "const KEY = \"data-warp-key\"; // a list item's element\nconst LINES = `a; // kept\n b`;\n";
	assert_eq!(warp::web::site::compacted(script), "const KEY = \"data-warp-key\";\nconst LINES = `a; // kept\n b`;\n");
}

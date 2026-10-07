// card web-bundle: what a hello-world site sends, gzipped, stays within its budget; the test prints each file so the
// suite's log tracks the sizes (notes/web_framework.md step 17). Svelte's hello world is about 3 KB, Solid's about 5 KB.
use std::io::Write;

const HELLO: &str = "p{ \"hello world\" }";
const FOLDER: &str = "scratch/bundle_budget";
/// 2026-10-07: 46.2 KB (host.js 23.8, app.wasm 17.5, markup.js 2.4, site.js 1.1, reader.js 1.1, index.html 0.2);
/// 40.5 KB with the case table as ranges (app.wasm 11.9)
const BUDGET_GZIPPED_BYTES: usize = 41_000;

fn gzipped_size(bytes: &[u8]) -> usize {
	let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
	encoder.write_all(bytes).expect("gzip");
	encoder.finish().expect("gzip").len()
}

#[test]
fn a_hello_world_site_stays_within_its_budget() {
	let site = warp::site::build(HELLO, "hello", std::path::Path::new(FOLDER)).expect("the site");
	let sizes: Vec<(String, usize)> = site.files.iter().map(|name| (name.clone(), gzipped_size(&std::fs::read(site.directory.join(name)).expect("a file")))).collect();
	let total: usize = sizes.iter().map(|(_, size)| size).sum();
	sizes.iter().for_each(|(name, size)| println!("hello-world site: {name:<12} {size:>7} bytes gzipped"));
	println!("hello-world site: total        {total:>7} bytes gzipped (budget {BUDGET_GZIPPED_BYTES})");
	assert!(total <= BUDGET_GZIPPED_BYTES, "a hello-world site is {total} bytes gzipped, over its budget of {BUDGET_GZIPPED_BYTES}");
}

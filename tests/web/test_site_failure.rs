// a site or Worker that does not compile says why, as the CLI does: the error and its excerpt, without the "nothing to
// compile:" of a program whose value is all it has (src/host/host_parts.rs compile_failure; the dev overlay shows it)
const TYPO: &str = "count = 3\ndiv{ p{ \"count \" + cuont } }";

#[test]
fn a_site_that_does_not_compile_says_why() {
	let Err(failure) = warp::web::site::files(TYPO, "typo", true) else {
		panic!("the site of a typo builds");
	};
	assert!(failure.starts_with("undefined variable: cuont"), "{failure}");
	assert!(failure.contains("2 | div{ p{ \"count \" + cuont } }"), "{failure}");
}

// card pannous-com: `://host/path` is a URL that takes the page's protocol, as `//host/path` does in HTML; natively https
use crate::is;

const LIBC_HEADER: &str = "://warp.pannous.com/lib/libc.h";

#[test]
fn a_url_may_start_at_its_colon_slash_slash() {
	is!(&format!("x = {LIBC_HEADER}\nx"), LIBC_HEADER);
}

#[test]
#[cfg(feature = "native")] // a real request over the network
fn natively_it_is_fetched_over_https() {
	is!(&format!("header = fetch \"{LIBC_HEADER}\"\nheader.contains(\"size_t\")"), true);
	is!(&format!("header = fetch {LIBC_HEADER}\nheader.contains(\"size_t\")"), true);
}

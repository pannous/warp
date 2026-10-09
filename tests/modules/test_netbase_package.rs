// Card netbase-package (notes/netbase.md): `use netbase` (lib/extra/netbase.warp) queries a netbase server over HTTP; without
// one running, the failure names the URL it tried. A test against a running netbase waits for its backend decision.
#[cfg(feature = "native")]
use crate::common::fails_with;
use crate::is;

#[test]
fn a_query_is_a_url_path() {
	is!("use netbase; netbase_path(\"all cities with population > 100000\")", "all%20cities%20with%20population%20%3E%20100000");
}

#[cfg(feature = "native")]
#[test]
fn without_a_server_the_failure_names_it() {
	fails_with("use netbase; query_at(\"http://localhost:1\", \"all cities\")", "http://localhost:1/json/query/all%20cities");
}

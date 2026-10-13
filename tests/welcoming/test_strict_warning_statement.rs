//! card under-use: under `use strict` a warning stops the run as `raise` does, also as a statement whose value is
//! dropped; the error names the warning's line and column
use crate::is;
use warp::error;

#[test]
fn a_strict_warning_before_the_last_statement_stops_the_run() {
	is!("use strict; warning \"careful\"; 1", error("careful at 1:13"));
	is!("use strict; warn \"careful\"; 1", error("careful at 1:13"));
	is!("use strict\nx = 1\nwarning \"careful\"\n2", error("careful at 3:1"));
	is!("use strict; try { warning \"careful\"; 1 } else 2", 2);
}

#[test]
fn without_strict_the_warning_continues() {
	is!("warning \"careful\"; 1", 1);
}

//! Card error-if: `error "?"` makes an error value; a statement that drops it warns toward `fail "?"`, which raises it
use crate::common::{fails_with, warns_with};
use crate::is;
use warp::errors::diagnostic::{with_warning_mode, WarningMode};

#[test]
fn a_discarded_error_warns() {
	warns_with("error \"?\"; 3", 3, "fail \"?\"");
	warns_with("x = 1; if x > 0 { error \"big\" }; 4", 4, "fail \"big\"");
	warns_with("x = 1; if x > 5 { 1 } else { error \"small\" }\n4", 4, "fail \"small\"");
}

#[test]
fn a_used_error_does_not_warn() {
	with_warning_mode(WarningMode::Error, || fails_with("f(x) := error \"bad\"; f(1)", "bad"));
	with_warning_mode(WarningMode::Error, || is!("e = error \"?\"; e failed", true));
	with_warning_mode(WarningMode::Error, || fails_with("error \"last\"", "last"));
}

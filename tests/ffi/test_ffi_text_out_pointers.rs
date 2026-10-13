// card ffi-header: a `char **` out-pointer (strtod's endptr) is left out of the warp call, and the C result stays
// the result; only an out-pointer to a struct (`sqlite3 **ppDb`) gives its handle as the result (notes/ffi_handles.md)
use crate::is;

#[test]
fn a_text_out_pointer_keeps_the_c_result() {
	is!("import strtod from 'c'\nstrtod('2.5')", 2.5);
	is!("import strtol from 'c'\nstrtol('ff', 16)", 255);
}

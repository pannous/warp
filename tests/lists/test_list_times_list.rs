//! card g_n8GI: `xs * ys` of two lists is their inner product, `dot(xs, ys)`; `xs * 2` stays an error (D3)
use crate::is;
use crate::common::fails_with;

#[test]
fn list_times_list_is_the_dot_product() {
	is!("xs=[1 2 3]; ys=[4 5 6]; xs * ys", 32);
	is!("xs=[1 2 3]; ys=[4 5 6]; d = dot(xs, ys); d1 = xs * ys; assert d1 == d; d1", 32);
	is!("[1 2] * [3 4]", 11);
	is!("xs = [1.5 2.5]; ys = [2.0 2.0]; xs · ys", 8.0);
	fails_with("xs = [1 2 3]; xs * 2", "ambiguous");
}

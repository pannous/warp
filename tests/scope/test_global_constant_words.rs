use warp::is;

#[test]
fn global_with_every_constant_word() {
	is!("global final x=7; x", 7);
	is!("global constant x=7; x", 7);
	is!("global val int x=7; x", 7);
}

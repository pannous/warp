use warp::is;

#[test]
fn function_defines_like_def() {
	is!("function square(n){ n*n }; square 3", 9);
	is!("function square (n: number){ n*n }; square 3", 9);
	is!("function square (n: number){ n*n }; square(4)", 16);
	is!("def square(n){ n*n }; square 3", 9);
}

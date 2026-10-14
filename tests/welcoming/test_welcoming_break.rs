//! `break` and `continue` (Crystal's `next`) leave or skip the innermost while/for loop; outside a loop they are a compile error

use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn break_leaves_the_innermost_loop() {
	is!("i=0; while i<10 { i=i+1; if i==3 { break } }; i", 3);
	is!("i=0\nwhile i<10 { i=i+1; if i==3 { break } }\ni", 3);
	is!("i=0\nwhile i<10 {\n\ti += 1\n\tif i == 3: break\n}\ni", 3);
	is!("n=0; while true { n+=1; if n>=7 { break } }; n", 7);
	is!("s=0; for i in 0..10 { if i==4 { break }; s+=i }; s", 6);
	is!("n=0\nfor i in 0..3 {\n  for j in 0..10 {\n    if j==2 { break }\n    n+=1\n  }\n}\nn", 6);
	is!("first_over(limit) := { r=0; for x in [3,8,12,20] { if x>limit { r=x; break } }; r }\nfirst_over(10)", 12);
}

#[test]
fn continue_skips_to_the_next_pass_and_keeps_the_for_step() {
	is!("i=0; s=0; while i<10 { i=i+1; if i%2==0 { continue }; s+=i }; s", 25);
	is!("s=0; for i in 0..10 { if i%2==1 { continue }; s+=i }; s", 20);
	is!("s=0; for x in [1,2,3,4,5] { if x==2 { continue }; if x==5 { break }; s+=x }; s", 8);
	is!("s=0; for(i=0;i<6;i++){ if i==2 { continue }; s+=i }; s", 13);
	is!(r#"s=""; for i in 0..4 { if i==2 { continue }; s += "x" }; s"#, "xxx");
	is!("found=0; for i in 1..5 { for j in 1..5 { if j==i { continue }; if i*j==6 { found=i*10+j; break } }; if found>0 { break } }; found", 23);
}

#[test]
fn next_is_continue_unless_it_names_a_variable() {
	is!("s=0; for i in 1..6 { if i==3 { next }; s+=i }; s", 12);
	is!("next=5; s=0; for i in 0..3 { s += next }; s", 15);
}

#[test]
fn break_outside_a_loop_is_a_compile_error() {
	let result = eval("x=1; break; x").to_string();
	assert!(result.contains("outside of a loop"), "{result}");
}

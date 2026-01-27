use warp::analyzer::analyze;
use warp::type_kinds::NodeKind;
use warp::wasp_parser::parse;
use warp::*;
use warp::node::strings;

// Array size tests
#[test]
#[ignore]
fn test_array_size() {
	// todo!
	// There should be one-- and preferably only one --obvious way to do it.
	// requires struct lookup and aliases
	is!("pixel=[1 2 4];#pixel", 3);
	//  is!("pixel=[1 2 4];pixel#", 3);
	is!("pixel=[1 2 4];pixel size", 3);
	is!("pixel=[1 2 4];pixel length", 3);
	is!("pixel=[1 2 4];pixel count", 3);
	is!("pixel=[1 2 4];pixel number", 3); // ambivalence with type number!
	is!("pixel=[1 2 4];pixel.size", 3);
	is!("pixel=[1 2 4];pixel.length", 3);
	is!("pixel=[1 2 4];pixel.count", 3);
	is!("pixel=[1 2 4];pixel.number", 3); // ambivalence cast
	is!("pixels=[1 2 4];number of pixels ", 3);
	is!("pixels=[1 2 4];size of pixels ", 3);
	is!("pixels=[1 2 4];length of pixels ", 3);
	is!("pixels=[1 2 4];count of pixels ", 3);
	is!("pixel=[1 2 3];pixel.add(5);#pixel", 4);
	// Edge cases: single element size
	is!("pixel=[5];#pixel", 1);
	is!("pixel=[5];pixel.size", 1);
	is!("pixel=[5];pixel.length", 1);
	// Edge cases: two element size
	is!("pixel=[5 7];#pixel", 2);
	is!("pixel=[5 7];pixel.length", 2);
	// Edge cases: large list size
	is!("pixel=[1 2 3 4 5 6 7 8 9 10];#pixel", 10);
}

#[test]
#[ignore]
fn test_array_operations() {
	// todo!
	test_array_size();
	// todo 'do' notation to modify versus return different list!
	is!("pixel=[1 2 3];do add 4 to pixel; pixel", ints(vec![1, 2, 3, 4]));
	is!("pixel=[1 2 3];y=pixel + 4; pixel", ints(vec![1, 2, 3]));

	//        throws("pixel=[1 2 3];pixel + 4;pixel");// unused non-mutating operation
	is!("pixels=[1 2 4];pixel#3", 4); // plural!
	is!("pixel=[1 2 3];pixel + [4]", ints(vec![1, 2, 3, 4]));
	is!("pixel=[1 2 3];pixel + 4", ints(vec![1, 2, 3, 4]));
	is!("pixel=[1 2 3];pixel<<4", ints(vec![1, 2, 3, 4]));
	// is!("pixel=[1 2 3];4>>pixel", ints(vec![4, 1, 2, 3]));
	is!("pixel=[1 2 3];add(pixel, 4)", ints(vec![1, 2, 3, 4])); // julia style
	is!("pixel=[1 2 3];add 4 to pixel", ints(vec![1, 2, 3, 4]));
	is!("pixel=[1 2 3];pixel.add 4", ints(vec![1, 2, 3, 4]));
	is!("pixel=[1 2 3];pixel add 4", ints(vec![1, 2, 3, 4]));
	is!("pixel=[1 2 3];pixel.add(4)", ints(vec![1, 2, 3, 4]));
	is!("pixel=[1 2 3];pixel.insert 4", ints(vec![1, 2, 3, 4]));
	is!("pixel=[1 2 3];pixel insert 4", ints(vec![1, 2, 3, 4]));
	is!("pixel=[1 2 3];pixel.insert(4)", ints(vec![1, 2, 3, 4]));
	is!("pixel=[1 2 3];pixel.insert(4,-1)", ints(vec![1, 2, 3, 4]));
	is!("pixel=[1 2 3];pixel.insert 4 at end", ints(vec![1, 2, 3, 4]));
	is!("pixel=[1 2 3];pixel.insert 4 at -1", ints(vec![1, 2, 3, 4]));
	is!("pixel=[1 2 3];insert 4 at end of pixel", ints(vec![1, 2, 3, 4]));
	is!("pixel=[1 2 3];pixel.insert(4,0)", ints(vec![4, 1, 2, 3]));
	is!("pixel=[1 2 3];pixel.insert 4 at 0", ints(vec![4, 1, 2, 3]));
	is!("pixel=[1 2 3];pixel.insert 4 at start", ints(vec![4, 1, 2, 3]));
	is!("pixel=[1 2 3];pixel.insert 4 at head", ints(vec![4, 1, 2, 3]));
	is!(
		"pixels=[1 2 3];insert 4 at start of pixels",
		ints(vec![4, 1, 2, 3])
	);
	is!("pixel=[1 2 3];pixel - [3]", ints(vec![1, 2]));
	is!("pixel=[1 2 3];pixel - 3", ints(vec![1, 2]));
	is!("pixel=[1 2 3];remove [3] from pixel", ints(vec![1, 2]));
	is!("pixel=[1 2 3];remove 3 from pixel", ints(vec![1, 2]));
	is!("pixel=[1 2 3];pixel.remove(3)", ints(vec![1, 2]));
	is!("pixel=[1 2 3];pixel.remove 3", ints(vec![1, 2]));
	is!("pixel=[1 2 3];pixel remove(3)", ints(vec![1, 2]));
	is!("pixel=[1 2 3];pixel remove 3", ints(vec![1, 2]));
	is!("pixel=[1 2 3];pixel.remove([3])", ints(vec![1, 2]));
	is!("pixel=[1 2 3];pixel.remove [3]", ints(vec![1, 2]));
	is!("pixel=[1 2 3];pixel remove([3])", ints(vec![1, 2]));
	is!("pixel=[1 2 3];pixel remove [3]", ints(vec![1, 2]));
	is!("pixel=[1 2 3 4];pixel.remove([3 4])", ints(vec![1, 2]));
	is!("pixel=[1 2 3 4];pixel.remove [3 4]", ints(vec![1, 2]));
	is!("pixel=[1 2 3 4];pixel remove([3 4])", ints(vec![1, 2]));
	is!("pixel=[1 2 3 4];pixel remove [3 4]", ints(vec![1, 2]));
	is!("pixel=[1 2 3 4];pixel remove 3 4", ints(vec![1, 2]));
	is!("pixel=[1 2 3 4];pixel remove (3 4)", ints(vec![1, 2]));
	is!("pixels=[1 2 3 4];pixels without (3 4)", ints(vec![1, 2]));
	// Edge cases: operations on single element lists
	is!("pixel=[5];pixel + 10", ints(vec![5, 10]));
	is!("pixel=[5];pixel - 5", ints(vec![]));
	is!("pixel=[5];pixel.insert(10,0)", ints(vec![10, 5]));
	// Edge cases: operations on two element lists
	is!("pixel=[1 2];pixel + 3", ints(vec![1, 2, 3]));
	is!("pixel=[1 2];pixel - 1", ints(vec![2]));
	is!("pixel=[1 2];pixel - 2", ints(vec![1]));
	// Edge cases: remove first and last elements
	is!("pixel=[1 2 3];pixel - 1", ints(vec![2, 3]));
	is!("pixel=[1 2 3];remove 1 from pixel", ints(vec![2, 3]));
	// Edge cases: insert at middle position
	is!("pixel=[1 3];pixel.insert(2,1)", ints(vec![1, 2, 3]));
}

#[test]
#[ignore]
fn test_array_creation() {
	//    skip!(

	// todo create empty array
	is!("pixel=[];pixel[1]=15;pixel[1]", 15);
	is!("pixel=();pixel#1=15;pixel#1", 15); // diadic ternary operator
	is!("pixel array;pixel#1=15;pixel#1", 15);
	is!("pixel:int[100];pixel[1]=15;pixel[1]", 15);
	is!("pixel=int[100];pixel[1]=15;pixel[1]", 15); // todo wasp can't distinguish type ':' from value '=' OK?
	is!("pixel: 100 int;pixel[1]=15;pixel[1]", 15); // number times type = typed array
	// Edge cases: minimal sized arrays
	is!("pixel:int[1];pixel[0]=5;pixel[0]", 5);
	is!("pixel:int[2];pixel[1]=10;pixel[1]", 10);
	// Edge cases: boundary access
	is!("pixel:int[10];pixel[0]=1;pixel[0]", 1);
	is!("pixel:int[10];pixel[9]=99;pixel[9]", 99);
}

#[test]
// #[ignore]
fn test_index_offset() {
	is!("(2 4 3)[1]", 4);
	is!("(2 4 3)#2", 4);
	is!("y=(1 4 3)#2", 4);
	is!("y=(1 4 3)[1]", 4);
	// Edge cases: first and last element
	is!("(2 4 3)[0]", 2);
	is!("(2 4 3)#1", 2);
	is!("(2 4 3)[2]", 3);
	is!("(2 4 3)#3", 3);
	// Single element list
	// is!("(5)[0]", 5); // TODO: single-element list indexing not working
	// is!("(5)#1", 5); // TODO: single-element list indexing not working
	// Two element list boundaries
	is!("(7 9)[0]", 7);
	is!("(7 9)[1]", 9);
}

#[test]
fn test_index_offset_advanced() {
	// List variable storage and indexing
	is!("x=(1 4 3);x#2=5;x#2", 5); // index assignment
	is!("x=(1 4 3);z=(9 8 7);x#2", 4); // list variable storage
	is!("x=(5 6 7);y=(1 4 3);y#2", 4); // list variable storage
	is!("x=(5 6 7);(1 4 3)#2", 4);
	is!("y=(1 4 3);y[1]", 4); // CAN NOT WORK in data_mode because y[1] ≈ y:1 setter
	is!("x=(5 6 7);y=(1 4 3);y[1]", 4);
	is!("(5 6 7);(2 4 3)[0]", 2);
	is!("x=(5 6 7);y=(1 4 3);y#2", 4); // list variable storage
	is!("(5 6 7);(1 4 3)#2", 4);
	is!("x=(5 6 7);(1 4 3)#2", 4); // list variable storage affects later stmt
	is!("puts('ok');(1 4 3)#2", 4);
	// Arithmetic (unrelated to indexing)
	is!("i=10007;x=i%10000", 7);
	is!("maxi=3840*2160", 3840 * 2160);
	is!("i=10007;x=i%10000", 7);
	// Index assignment with variable index
	is!("k=(1,2,3);i=1;k#i=4;k#1", 4); // index assignment with variable
	is!("k=(1,2,3);i=1;k#i=4;k#1", 4); // index assignment with variable
									// More list tests
	is!("x=(1 4 3);x#2=5;x#2", 5); // index assignment
	is!("x=(1 4 3);x#2", 4); // list variable storage
	// Edge cases: boundary assignments
	is!("x=(1 4 3);x#1=9;x#1", 9); // first element assignment
	is!("x=(1 4 3);x#3=9;x#3", 9); // last element assignment
	// is!("x=(5);x#1=7;x#1", 7); // TODO: single element list assignment not working
	// Variable index at boundaries
	is!("k=(1,2,3);i=0;k#i", 1); // index 0 with variable
	is!("k=(1,2,3);i=3;k#i", 3); // last index with variable
}

#[test]
#[ignore = "while loop with nop needs investigation"]
fn test_while_nop_issue() {
	// This test was failing before list indexing changes
	is!("x=0;while x++<11: nop;", 11);
}

#[test]
#[ignore]
fn test_array_initialization_basics() {
	// via Units
	let node = analyze(parse("x : 100 numbers"));
	eq!(node.kind(), NodeKind::List);
	eq!(node.length(), 100);
}

#[test]
#[ignore]
fn test_array_initialization() {
	// via Units
	is!("x : int[100]; x.length", 100);
	//     is!("x : u8 * 100; x.length", 100) // type times size operation!!
	is!("x : 100 * int; x.length", 100);
	is!("x : 100 * ints;[ x.length", 100);
	//     is!("x : 100 ints;[ x.length", 100) // implicit multiplication, no special case!
	is!("x : 100 int; x.length", 100);
	is!("x : 100 integers; x.length", 100);
	is!("x : 100 numbers; x.length", 100);
	is!("x is 100 times [0]; x.length", 100);
	is!("x is array of size 100; x.length", 100);
	is!("x is an 100 integer array; x.length", 100);
	is!("x is a 100 integer array; x.length", 100);
	is!("x is a 100 element array; x.length", 100);
	// Edge cases: small sizes
	is!("x : int[1]; x.length", 1);
	is!("x : 1 int; x.length", 1);
	is!("x : int[2]; x.length", 2);
	is!("x : 2 integers; x.length", 2);
	// Edge cases: larger sizes
	is!("x : int[1000]; x.length", 1000);
	is!("x : 1000 numbers; x.length", 1000);
}

#[test]
#[ignore]
fn test_array_indices() {
	// #[cfg(not(feature = "WASM"))]{
	//         ( and INCLUDE_MERGER);
	is!("(1 4 3)#2", 4); // todo needs_runtime = true => whole linker machinery
	is!("x=(1 4 3);x#2", 4);
	is!("x=(1 4 3);x#2=5;x#2", 5);
	// Edge cases: first and last
	is!("(1 4 3)#1", 1);
	is!("(1 4 3)#3", 3);
	is!("x=(1 4 3);x#1", 1);
	is!("x=(1 4 3);x#3", 3);
	// Single element
	is!("(7)#1", 7);
	is!("x=(7);x#1", 7);
	is!("x=(7);x#1=9;x#1", 9);
	// }
}

#[test]
fn test_root_lists() {
	// vargs needs to be 0 terminated, otherwise pray!
	is!("1 2 3", ints(vec![1, 2, 3]));
	is!("(1 2 3)", ints(vec![1, 2, 3]));
	is!("(1,2,3)", ints(vec![1, 2, 3]));
	is!("(1;2;3)", ints(vec![1, 2, 3]));
	//     is!("1;2;3", ints(vec![1, 2, 3, 0]) //ok
	is!("1,2,3", ints(vec![1, 2, 3]));
	is!("[1 2 3]", ints(vec![1, 2, 3]));
	is!("[1 2 3]", ints(vec![1, 2, 3]));
	is!("[1,2,3]", ints(vec![1, 2, 3]));
	is!("[1,2,3]", ints(vec![1, 2, 3]));
	is!("[1;2;3]", ints(vec![1, 2, 3]));
	is!("{1 2 3}", ints(vec![1, 2, 3]));
	is!("{1,2,3}", ints(vec![1, 2, 3]));
	is!("{1;2;3}", ints(vec![1, 2, 3]));
	// Single element lists
	// is!("(1)", ints(vec![1])); // TODO: (n) is treated as grouping, not single-element list
	// is!("[1]", ints(vec![1])); // TODO: single-element list not working properly
	// is!("{1}", ints(vec![1])); // TODO: single-element list not working properly
	// Two element lists
	is!("(1 2)", ints(vec![1, 2]));
	is!("[1,2]", ints(vec![1, 2]));
	is!("{1;2}", ints(vec![1, 2]));
	// Large list
	is!("(1 2 3 4 5 6 7 8 9 10)", ints(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]));
	// Negative numbers
	// is!("(-1 -2 -3)", ints(vec![-1, -2, -3])); // TODO: negatives in lists treated as subtraction
	// is!("[-1,-2,-3]", ints(vec![-1, -2, -3])); // TODO: negatives in lists treated as subtraction
	// Mixed positive and negative
	// is!("(1 -2 3)", ints(vec![1, -2, 3])); // TODO: negatives in lists treated as subtraction
	// Zero handling
	// is!("(0)", ints(vec![0])); // TODO: (n) is treated as grouping, not single-element list
	is!("(0 0 0)", ints(vec![0, 0, 0]));
	is!("(0 1 2)", ints(vec![0, 1, 2]));
}

#[test]
fn test_root_list_strings() {
	is!("(a,b,c)", strings(vec!["a", "b", "c"])); // symbols shall match string
	is!("(a;b;c)", strings(vec!["a", "b", "c"]));
	is!("a;b;c", strings(vec!["a", "b", "c"]));
	is!("a,b,c", strings(vec!["a", "b", "c"]));
	is!("{a b c}", strings(vec!["a", "b", "c"]));
	is!("{a,b,c}", strings(vec!["a", "b", "c"]));
	is!("[a,b,c]", strings(vec!["a", "b", "c"]));
	is!("(a b c)", strings(vec!["a", "b", "c"]));
	is!("[a;b;c]", strings(vec!["a", "b", "c"]));
	is!("a b c", strings(vec!["a", "b", "c"]));
	is!("{a;b;c}", strings(vec!["a", "b", "c"]));
	is!("[a b c]", strings(vec!["a", "b", "c"]));
	// Single element string lists
	// is!("(x)", strings(vec!["x"])); // TODO: (n) is treated as grouping, not single-element list
	// is!("[x]", strings(vec!["x"])); // TODO: single-element list not working properly
	// is!("{x}", strings(vec!["x"])); // TODO: single-element list not working properly
	// Two element string lists
	is!("(a b)", strings(vec!["a", "b"]));
	is!("[a,b]", strings(vec!["a", "b"]));
	is!("{a;b}", strings(vec!["a", "b"]));
}

#[test]
	fn test_index() {
	let result = parse("[a b c]#2");
	// eqs!(result, "[a  b  c]#2");
	eq!(result.first().length(), 3); // (# [a b c] 2)
	is!("(a b c)#2", "b"); // or compile time list indexing??
	is!("{a b c}#2", "b");
	is!("[a b c]#2", "b");
	// Boundary indexing
	is!("(a b c)#1", "a"); // first element
	is!("(a b c)#3", "c"); // last element
	is!("[a b c]#1", "a");
	is!("[a b c]#3", "c");
	is!("{a b c}#1", "a");
	is!("{a b c}#3", "c");
	// Single element indexing
	is!("(x)#1", "x");
	is!("[x]#1", "x");
	is!("{x}#1", "x");
	// Two element list indexing
	is!("(x y)#1", "x");
	is!("(x y)#2", "y");
	skip!(
			is!("{a:1 b:2}.a", 1);
			is!("a of {a:1 b:2}", 1);
			is!("a in {a:1 b:2}", 1);
			is!("{a:1 b:2}[a]", 1);
			is!("{a:1 b:2}.b", 2);
			is!("b of {a:1 b:2}", 2);
			is!("b in {a:1 b:2}", 2);
			is!("{a:1 b:2}[b]", 2);
	);
}

#[allow(dead_code)]
#[test]
fn test_indexed() {
	println!("test_indexed own extension indexed for Vec<i32>");
	let v = vec![1, 2, 3, 4, 5];
	for (i, item) in v.indexed() {
		println!("{}: {}", i, item);
		eq!(item, i + 1);
	}
	// Edge case: single element
	let single = vec![10];
	for (i, item) in single.indexed() {
		eq!(i, 0);
		eq!(item, 10);
	}
	// Edge case: two elements
	let pair = vec![7, 9];
	for (i, item) in pair.indexed() {
		println!("{}: {}", i, item);
		eq!(item, if i == 0 { 7 } else { 9 });
	}
}

#[test]
fn test_filter() {
	println!("test_filter own extension filter for Vec<i32>");
	let v = vec![1, 2, 3, 4, 5];
	for i in v.clone().filter(|&x| x > 2) {
		print!("{} ", i);
	}
	// let xs = Node::list(v);
	let xs = ints(v);
	for node in xs.filter(|x| x > &2) {
		print!("{} ", node);
	}
	// Edge cases: filter on single element
	let single = ints(vec![5]);
	for node in single.filter(|x| x > &2) {
		print!("{} ", node);
	}
	// Edge cases: filter on two elements
	let pair = ints(vec![1, 5]);
	for node in pair.filter(|x| x > &2) {
		print!("{} ", node);
	}
	// Edge cases: filter none match
	let none = ints(vec![1, 2]);
	for node in none.filter(|x| x > &10) {
		print!("{} ", node);
	}
	// Edge cases: filter all match
	let all = ints(vec![5, 6, 7]);
	for node in all.filter(|x| x > &2) {
		print!("{} ", node);
	}
}

#[test]
#[ignore] // TODO: Node iteration needs type coercion
fn test_iteration() {
	let xs = ints(vec![1, 2, 3]);
	let mut count = 0;
	for _x in xs {
		count += 1;
	}
	eq!(count, 3);
}

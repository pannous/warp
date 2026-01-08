use warp::skip;
use warp::extensions::todow;
use warp::node::data;
use warp::wasp_parser::parse;
use warp::*;

#[test]
fn test_add_boxed_list_item() {
	let list = ints![1, 2];
	let num = int(3);
	let result = list.add(num);
	assert_eq!(result, ints![1, 2, 3]);
	assert_eq!(ints![1, 2, 3], result);
}


#[test]
#[ignore] // shall this ever work?
fn test_add_pure_vec_data() {
	let list = data(vec![1, 2]); // only via Dada!
	assert_eq!(ints![1, 2], list); // nah, not even this works (for good reason) or to do?
	assert_eq!(list, ints![1, 2]);
	let num = int(3);
	let result = list.add(num); // Still needs to be wrapped and probably doesn't work.
	assert_eq!(result, ints![1, 2, 3]);
	assert_eq!(ints![1, 2, 3], result);
}

// #[test]
// fn test_add_pure_list_item() {
// 	let list = list(vec![1, 2]); // only via Dada!
// 	list.add(3)
// 	assert_eq!(result, ints![1, 2, 3]);
// 	assert_eq!(ints![1, 2, 3], result);
// }


#[test]
fn test_add_list_item_explicit() {
	let list = Node::list(vec![Node::int(1), Node::int(2)]);
	let num = Node::int(3);

	// List + item should append
	let result = list.add(num);
	assert_eq!(
		result,
		Node::list(vec![Node::int(1), Node::int(2), Node::int(3)])
	);
}

#[test]
fn test_add_item_list() {
	let num = Node::int(0);
	let list = Node::list(vec![Node::int(1), Node::int(2)]);

	// item + List should prepend
	let result = num.add(list);
	assert_eq!(
		result,
		Node::list(vec![Node::int(0), Node::int(1), Node::int(2)])
	);
}

#[test]
fn test_add_text() {
	let text1 = Node::text("hello");
	let text2 = Node::text("world");

	let result = text1.add(text2);
	assert_eq!(result, "helloworld");
}

#[test]
fn test_add_numbers() {
	let num1 = Node::int(5);
	let num2 = Node::int(7);

	let result = num1.add(num2);
	assert_eq!(result, 12);
}

#[test]
fn test_add_lists() {
	let list1 = Node::list(vec![Node::int(1), Node::int(2)]);
	let list2 = Node::list(vec![Node::int(3), Node::int(4)]);

	let result = list1.add(list2);
	assert_eq!(
		result,
		Node::list(vec![Node::int(1), Node::int(2), Node::int(3), Node::int(4)])
	);
}

#[test]
fn test_add_with_meta() {
	let num1 = Node::int(5).with_comment("first".to_string());
	let num2 = Node::int(7);
	// Meta on left should unwrap and add
	let result = num1.add(num2);
	assert_eq!(result, 12);
}

#[test]
fn test_add_meta_on_right() {
	let num1 = Node::int(5);
	let num2 = Node::int(7).with_comment("second".to_string());
	let result = num1.add(num2);
	assert_eq!(result, 12);
}

#[test]
fn test_while_true_forever() {
	todow("test_while_true_forever");

	skip!(
		is!("def stop():{0};while !stop() : {}", 0); // should hang forever
		is!("def goo():{1};while goo() : {}", 0); // should hang forever
		is!("while True : 2", 0); // should hang forever
	);
}

#[test]
fn test_key() {
	let node = parse("x:40;x+1");
	assert!(node.length() == 2);
	assert!(node[0]["x"] == 40);
}

#[test]
fn test_for_each() {}

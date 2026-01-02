use wasp::node::Node;

#[test]
fn test_add_list_item() {
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

    // Meta on right should unwrap and add
    let result = num1.add(num2);
    assert_eq!(result, 12);
}

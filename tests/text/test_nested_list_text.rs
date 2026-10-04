// The text of a list of lists is its literal, at run time too; `#m#1` counts the first row
use warp::is;

#[test]
fn a_nested_list_has_the_text_of_its_literal() {
	is!("t = [[1 2] [3]]; str(t)", "[[1 2] [3]]");
	is!("f() := [[1 2] [3]]; str(f())", "[[1 2] [3]]");
	is!("t = [[1 2] [3 [4 5]]]; str(t)", "[[1 2] [3 [4 5]]]");
	is!("transpose(m) := { tt = []; for c in 1..#m#1+1 { row = []; for r in 1..#m+1 { row.add(m#r#c) }; tt.add(row) }; tt }; str(transpose([[1 2 3] [4 5 6]]))", "[[1 4] [2 5] [3 6]]");
}

#[test]
fn a_count_takes_the_indexed_element() {
	is!("m = [[1 2 3] [4 5 6]]; #m#1", 3);
	is!("xs = [1 2]; #xs + 1", 3);
}

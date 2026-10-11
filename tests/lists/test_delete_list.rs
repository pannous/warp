use crate::is;

const DELETE_FROM_LIST: &str = include_str!("../../probes/delete_list.warp");

#[test]
fn delete_removes_the_first_matching_list_item() {
	is!(DELETE_FROM_LIST, warp::ints(vec![1, 3, 2]));
}

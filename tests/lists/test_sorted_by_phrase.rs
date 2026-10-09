// card sorted-by-phrase: `xs sorted by <key>` sorts by a key of each element `it`, as `xs.sort(by: it => key)`;
// `by -key` (read as `by - key`) sorts descending (found editing samples/orm.warp)
use crate::is;
use warp::ints;

#[test]
fn a_list_sorted_by_a_key_of_it() {
	is!("xs = [3, 1, 2]; xs sorted by it", ints(vec![1, 2, 3]));
	is!("xs = [3, 1, 2]; xs sorted by -it", ints(vec![3, 2, 1]));
	is!("xs = [3, 1, 2]; xs sort by -it", ints(vec![3, 2, 1]));
	is!("class P{name: text; age: int}; ps = [P(\"a\", 30), P(\"b\", 7)]; (ps sorted by it.age)#1.name", "b");
	is!("class P{name: text; age: int}; ps = [P(\"a\", 7), P(\"b\", 30)]; (ps sorted by -it.age).map(p => p.name)", warp::texts(vec!["b", "a"]));
}

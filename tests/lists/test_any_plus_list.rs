// card slice-node: a value known only at run time joined with a list (`parts + [symbol("a")]`, parts an any) holds
// items of any type: an item read, assigned or looped over keeps its kind instead of reading as an int
use crate::is;

#[test]
fn an_any_joined_with_a_list_keeps_its_items_kinds() {
	is!("f(n: any): any := { xs = n + [\"qq\"]; y = xs#2; y }; f([\"bb\"])", "qq");
	is!("f(n: any): text := { xs = [\"qq\"] + n; out = \"\"; for x in xs { out += x }; out }; f([\"bb\"])", "qqbb");
	is!("f(n: any): bool := { xs = n[0 ..< 1] + [symbol(\"a\")]; xs#2 is symbol }; f(parse(\"(b c d)\"))", true);
	is!("f(n: any): int := { xs = n + [1]; xs#2 + 1 }; f([5])", 2);
}

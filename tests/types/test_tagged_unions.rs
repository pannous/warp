// card tagged-unions: types with named variants carrying data (`enum Shape { Circle(r: float), Dot }`, Rust's enums,
// Swift's associated values, ML/Haskell sum types), matched by variant; exhaustiveness is card match-exhaustive
use crate::is;

const SHAPE: &str = "enum Shape { Circle(r: float), Square(side: float), Dot }";

fn with_shape(code: &str) -> String {
	format!("{SHAPE}; {code}")
}

#[test]
fn variants_carry_their_data() {
	is!(&with_shape("s = Circle(2.0); s.r"), 2.0);
	is!(&with_shape("s = Circle(2.0); s is Circle"), true);
	is!(&with_shape("s = Circle(2.0); s is Shape"), true);
	is!(&with_shape("Dot is Shape"), true);
}

#[test]
fn arrow_arms_bind_the_variant_data() {
	is!(
		&with_shape(
			"area(s: Shape) := match s { Circle(r) => 3*r*r; Square(a) => a*a; Dot => 0.0 }; area(Square(3.0))"
		),
		9.0
	);
	is!(&with_shape("s = Circle(2.0); match s { Shape::Circle(r) => r; _ => 0.0 }"), 2.0);
}

#[test]
fn colon_arms_bind_the_variant_data_too() {
	is!(&with_shape("area(s) := match s { Circle(r): 3*r*r; Square(a): a*a; Dot: 0.0 }; area(Circle(2.0))"), 12.0);
	is!(&with_shape("s = Square(3.0); match s { Dot: 0.0; Square(a): a*a; _: 1.0 }"), 9.0);
}

#[test]
fn variants_may_hold_the_union_itself() {
	is!(
		"enum Tree { Leaf(v: int), Node(l: Tree, r: Tree) }; sum(t) := match t { Leaf(v) => v; Node(l, r) => sum(l) + sum(r) }; sum(Node(Leaf(1), Node(Leaf(2), Leaf(3))))",
		6
	);
	is!(
		"enum Result { Ok(value: int), Err(message: text) }; r = Err(\"bad\"); match r { Ok(v): \"ok\"; Err(m): m }",
		"bad"
	);
}

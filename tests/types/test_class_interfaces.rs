// Interfaces, traits and protocols with classes (notes/traits.md "methods in the class body as sugar for the
// witnesses"): a class method satisfies a trait operation, a parameter typed by a trait takes any conforming instance,
// and the declarations as other languages write them
use crate::is;

const SHAPES: &str = "trait Shape { area }\nclass Sq{side:int; area() := side*side}\nclass Re{w:int; h:int; area() := w*h}\n";

#[test]
fn a_class_method_satisfies_a_trait_operation() {
	is!(&format!("{SHAPES}Sq(3).area()"), 9);
	is!(&format!("{SHAPES}f(s:Shape) := s.area()\nf(Sq(3))"), 9);
	is!(&format!("{SHAPES}f(s:Shape) := s.area()\nf(Sq(3)) + f(Re(2, 5))"), 19);
}

#[test]
fn interfaces_as_other_languages_write_them() {
	is!("interface Shape { double area(); }\nclass Square implements Shape {\n  double side;\n  Square(double s) { side = s; }\n  public double area() { return side * side; }\n}\nnew Square(3).area()", 9);
	is!("protocol Shape { func area() -> Double }\nstruct Square: Shape { var side: Double; func area() -> Double { side * side } }\nSquare(side: 3).area()", 9);
	is!("type Shape interface { Area() float64 }\ntype Square struct { Side float64 }\nfunc (s Square) Area() float64 { return s.Side * s.Side }\nfunc total(s Shape) float64 { return s.Area() }\ntotal(Square{Side: 3})", 9);
	is!("interface Shape { fun area(): Int }\nclass Square(val side: Int) : Shape { override fun area() = side * side }\nSquare(3).area()", 9);
	is!("interface Shape { area(): number }\nclass Square implements Shape {\n  side: number\n  constructor(side: number) { this.side = side }\n  area(): number { return this.side * this.side }\n}\nnew Square(3).area()", 9);
}

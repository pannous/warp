//! `type Point: {` with its fields on the lines below declares them, as `type Point {` does (the colon read the braces as
//! an indented block and nested them: 'Point has no field x'; samples/particles.warp)
use crate::is;

#[test]
fn a_type_colon_block_declares_its_fields() {
	is!("type P: {\n    x: float\n    y: float\n}\np = P {\n    x: 1.5\n    y: 2.0\n}\np.x", 1.5);
	is!("type P: { x: int y: int }; p = P{x: 1 y: 2}; p.y", 2);
}

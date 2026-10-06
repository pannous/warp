// A lambda's parameters may declare their types like a definition's: `(x:float)=>x/2` (it was no function at all:
// "cannot extract a numeric value from (x:float)=>x*1.5")
use crate::is;
use warp::*;

#[test]
fn a_lambda_parameter_declares_its_type() {
	is!("f = (x:float)=>x/2; f(3)", 1.5);
	is!("f = (x:int, y)=>x+y; f(2, 3)", 5);
	is!("f = (x:float)=>x*1.5; s = 0.0; for i in 0..4 { s += f(i) }; s", 9.0);
	is!("[1, 2].map((x:int)=>x*2)", list(vec![int(2), int(4)]));
}

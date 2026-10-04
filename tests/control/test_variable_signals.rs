// Signal keywords on variables (wiki/signal.md): `once x==5 {…}` listens to every later change of x and runs its body
// the first time the condition holds; `whenever` each time it holds after a change
use warp::is;

#[test]
fn once_runs_the_first_time_the_condition_holds() {
	is!("x=10; hits=0; once x==5 {hits+=1}; while x-->0 : x; hits", 1);
	is!("x=0; seen=0; once x>2 {seen=x}; x=1; x=3; x=7; seen", 3);
}

#[test]
fn once_never_runs_when_the_condition_never_holds() {
	is!("x=0; hits=0; once x==5 {hits+=1}; x=1; x=2; hits", 0);
}

#[test]
fn whenever_runs_each_time_the_condition_holds() {
	is!("x=0; hits=0; whenever x>1 {hits+=1}; x=1; x=2; x=3; x=0; hits", 2);
	is!("x=0; total=0; whenever x%2==0 : total+=x; for i in 1 to 6 { x = i }; total", 12);
}

#[test]
fn the_wiki_countdown() {
	is!("x=10\nonce x==5 {print \"countdown halfway done\"}\nwhile x-->0 : print x\nx", -1);
}

#[test]
fn on_set_runs_after_each_write_with_the_value() {
	is!("x=10; total=0; on set x {total+=value}; x=3; x=4; total", 7);
	is!("x=3; n=0; on set x : n+=1; while x-->0 : x; n", 4);
}

// `await go f(x)` awaits the task it starts (the await was dropped: the value was the task's number)
use warp::*;

#[test]
fn await_of_a_start_is_the_result() {
	is!("f(x) := x * 2; await go f(21)", 42);
	is!("f(x:float) := x * 2; await go f(1.5)", 3.0);
}

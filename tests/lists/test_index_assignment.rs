//! Element assignment `list[i]=x`, also by a variable index and inside while loops (condensed from probe_index_loop.rs;
//! the return-the-counter case is in test_todo.rs)
use crate::is;

#[test]
fn test_simple_index_assign() {
	is!("pixel=(1 2 3);pixel[1]=9;pixel[1]", 9);
}

#[test]
fn test_index_assign_with_variable() {
	is!("pixel=(1 2 3);i=1;pixel[i]=9;pixel[1]", 9);
}

#[test]
fn test_index_assign_in_while_loop() {
	is!("i=0;pixel=(0 0 0);while(i<3){pixel[i]=i;i=i+1};pixel[1]", 1);
}

#[test]
fn test_index_assign_in_while_with_increment() {
	is!("i=0;pixel=(0 0 0);while(i++<3){pixel[i]=i};pixel[2]", 2);
}


#[test]
fn test_index_assign_25_elements() {
	is!("i=0;w=5;h=5;pixel=(0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0);while(i++ < w*h){pixel[i]=i%2 };i ", 25);
}

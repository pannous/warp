// Signal keywords on variables (wiki/signal.md): `once x==5 {…}` listens to every later change of x and runs its body
// the first time the condition holds; `whenever` each time it holds after a change
use crate::is;

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

#[test]
fn after_a_function_runs_after_each_statement_calling_it() {
	is!("n=0; def test(x): x+1; after tested: n+=10; test(1); test(2); n", 20);
	is!("n=0; def stop(): 1; after stopped {n+=1}; for i in 1 to 3 { stop() }; n", 3);
	is!("log=0; def save(x): x; before save: log=log*10+1; after saved: log=log*10+2; save(5); log", 12);
}

// Derived signals (notes/signals.md phase 1): a `:=` value is a computed signal, a listener on it watches its sources
#[test]
fn a_listener_on_a_charged_value_watches_what_it_reads() {
	is!("a=1; b=1; total := a+b; hits=0; whenever total > 5 {hits+=1}; a=3; a=5; b=0; hits", 1);
	is!("a=1; twice := a*2; quad := twice*2; seen=0; once quad > 10 {seen=quad}; a=2; a=3; a=4; seen", 12);
}

#[test]
fn on_change_runs_only_when_the_value_differs() {
	is!("x=1; n=0; on change x : n+=1; x=1; x=2; x=2; x=3; n", 2);
	is!("a=1; b=2; sum := a+b; log=0; on change sum {log = log*10 + value}; a=2; a=2; b=1; log", 43);
}

#[test]
fn on_set_of_a_charged_value_listens_to_its_changes() {
	is!("a=1; total := a*2; n=0; on set total : n+=1; a=2; a=2; a=3; n", 2);
}

// P111: writes through `global x` inside called functions run the listeners (notes/signals.md phase 3)
#[test]
fn a_global_write_inside_a_function_runs_the_listeners() {
	is!("x=0; hits=0; once x==5 {hits+=1}; def f(){ global x; x=5 }; f(); hits", 1);
	is!("x=0; hits=0; def f(){ global x; x=5 }; once x==5 {hits+=1}; f(); hits", 1);
	is!("x=0; hits=0; whenever x>1 {hits+=1}; def f(v){ global x; x=v }; f(2); f(3); f(0); hits", 2);
	is!("x=0; log=0; on change x {log=log*10+value}; def store(v){ global x; x=v }; store(1); store(1); store(2); log", 12);
}

#[test]
fn a_global_write_before_the_listener_is_not_seen() {
	is!("x=0; hits=0; def f(){ global x; x=5 }; f(); once x==5 {hits+=1}; x=1; hits", 0);
}

#[test]
fn a_condition_ending_in_a_variable_keeps_its_block() {
	is!("a=1; b=2; hits=0; whenever a==b {hits+=1}; a=2; b=3; hits", 1);
}

// P112: no batching block; a multi-assignment notifies once, after all its writes (notes/signals.md phase 4)
#[test]
fn a_multi_assignment_notifies_once_after_all_its_writes() {
	is!("a=1; b=2; seen=0; whenever a==b {seen+=1}; a, b = 2, 1; seen", 0);
	is!("x=0; y=0; n=0; whenever x+y>0 {n+=1}; x, y = 1, 1; n", 1);
}

// A field or item write is a write of its variable (notes/signals.md phase 5)
#[test]
fn listeners_see_field_and_item_writes() {
	is!("person = {name:\"A\", age:30}; n=0; whenever person.age > 30 {n+=1}; person.age = 31; person.age = 29; n", 1);
	is!("p = {age:1, size:5}; log=0; on change p.age {log=log*10+value}; p.age = 2; p.age = 2; p.size = 6; p.age=3; log", 23);
	is!("xs=[1,2,3]; hits=0; whenever xs#1 > 5 {hits+=1}; xs#1 = 9; hits", 1);
}

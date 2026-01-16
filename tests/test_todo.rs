use warp::{is, Node};
// @claude once tests here are passing, move them to the appropriate correct test file!

#[test]
#[cfg(feature = "GRAFIX")]
fn test_paint_wasm() {
	//	struct timeval stop, start;
	//	gettimeofday(&start, NULL);
	// todo: let compiler compute constant expressions like 1024*65536/4
	//    	is!("i=0;k='hi';while(i<1024*65536/4){i++;k#i=65};k[1]", 65)// wow SLOOW!!!
	//out of bounds memory access if only one Memory page!
	//         is!("i=0;k='hi';while(i<16777216){i++;k#i=65};paint()", 0) // still slow, but < 1s
	// wow, SLOWER in wasm-micro-runtime HOW!?
	//	exit(0);

	//(√((x-c)^2+(y-c)^2)<r?0:255);
	//(x-c)^2+(y-c)^2
	is!("h=100;r=10;i=100;c=99;r=99;x=i%w;y=i/h;k=‖(x-c)^2+(y-c)^2‖<r", 1);
	////char *wasm_paint_routine = "urface=(1,2);i=0;while(i<1000000){i++;surface#i=i*(10-√i);};paint";
	//         char * wasm_paint_routine = "w=1920;c=500;r=100;surface=(1,2);i=0;"
	//         "while(i<1000000){"
	//         "i++;x=i%w;y=i/w;surface#i=(x-c)^2+(y-c)^2"
	"};paint";
	//((x-c)^2+(y-c)^2 < r^2)?0x44aa88:0xffeedd
	//char *wasm_paint_routine = "urface=(1,2);i=0;while(i<1000000){i++;surface#i=i;};paint";
	//is!(wasm_paint_routine, 0);
	//	char *wasm_paint_routine = "maxi=3840*2160/4/2;init_graphics();surface=(1,2,3);i=0;while(i<maxi){i++;surface#i=i*(10-√i);};";
	eval(wasm_paint_routine);
	//	paint(0);
	//	gettimeofday(&stop, NULL);
	//	printf!("took %lu µs\n", (stop.tv_sec - start.tv_sec) * 100000 + stop.tv_usec - start.tv_usec);
	//	printf!("took %lu ms\n", ((stop.tv_sec - start.tv_sec) * 100000 + stop.tv_usec - start.tv_usec) / 100);
	//	exit(0);
	//char *wasm_paint_routine = "init_graphics(); while(1){paint()}";// SDL bugs a bit
	//        while (1)paint(0);// help a little
}

// === If-then-else ===
#[test]
fn test_if_then_else() {
	is!("if 4>1 then 2 else 3", 2);
}

// === List indexing after puts ===
#[test]
fn test_list_index_after_puts() {
	is!("puts('ok');(1 4 3)#2", 4);
}

// === Square root arithmetic ===
#[test]
fn test_sqrt_arithmetic() {
	is!("3 + √9", 6);
}

// === Lambda/closure tests ===
#[test]
fn test_lambda_simple() {
	is!("grows:=it*2; grows 3", 6);
}

#[test]
fn test_lambda_with_multiply() {
	is!("grows:=it*2; grows 3*4", 24);
}

#[test]
fn test_lambda_comparison() {
	is!("grows:=it*2; grows(3*42) > grows 2*3", 1);
}

// === $0 parameter reference ===
#[test]
fn test_param_reference() {
	// $0 parameter reference (explicit param style with parentheses)
	is!("add1(x):=$0+1;add1(3)", 4);
}

// === Index assignment in loops (now working!) ===
#[test]
fn test_index_assign_in_loop() {
	// Index assignment with properly sized array
	is!("i=0;pixel=(0 0 0 0 0);while(i++<5){pixel[i]=i%2};i", 5);
}


#[test]
fn test_type() {
	// type() returns a Symbol with the type name
	is!("type(42)", Node::Symbol("int".to_string()));
	is!("type(3.14)", Node::Symbol("float".to_string()));
	is!("type('hello')", Node::Symbol("text".to_string()));
	// Type of inferred variable
	is!("x=42;type(x)", Node::Symbol("int".to_string()));
}

#[test]
#[ignore = "typed variable declaration tracking not yet implemented"]
fn test_type_annotated() {
	// Explicit type annotation - needs type tracking in scope
	is!("x:int=1;type(x)", Node::Symbol("int".to_string()));
}


#[test]
#[ignore = "todo"]
fn test_array_type_generics() {
	is!("pixels=(1,2,3);type(pixels)","list<int>");
}



#[test]
#[ignore = "array introspection functions not yet implemented"]
fn test_array_length() {
	is!("pixels=(1,2,3);#pixels", 3); // element count ⚠️ number operator ≠ index ≠ comment
	is!("pixels=(1,2,3);count(pixels)", 3); // element count
	is!("pixels=(1,2,3);pixels.count()", 3); // element count
	is!("pixels=(1,2,3);pixels.count", 3); // element count methods/getters don't need ()
	// is!("pixels=(1,2,3);pixel count", 3); // element count
	is!("pixels=(1,2,3);number of pixels", 3); // element count
	is!("pixels=(1,2,3);pixels.number()", 3); // element count
	is!("pixels=(1,2,3);size(pixels) ", 3 * 8); // ⚠️ byte count as i64
	// is!("pixels=(1,2,3);length(pixels) ", 3 * xyz); // ⚠️ byte count as node ???
}


#[test]
#[ignore = "typed array constructor not yet implemented"]
fn test_array_constructor() {
	is!("i=0;w=800;h=800;pixels=640000*int;size(pixels) ", 800 * 800 * 4); // byte count
	is!("i=0;w=800;h=800;pixels=640000*int;length(pixels) ", 800 * 800); // element count
}

// === Still pending (requires major features) ===
#[test]
#[ignore = "requires polymorphic function dispatch"]
fn test_polymorphic_dispatch() {
	is!("square(3.0)", 9.);
}

#[test]
#[ignore = "requires print function implementation"]
fn test_print_function() {
	is!("print 3", 3);
}

#[test]
#[ignore = "requires UTF-8 char indexing vs byte indexing"]
fn test_utf8_char_indexing() {
	// UTF-8 char indexing vs byte indexing (encoding redesign)
	is!("'αβγδε'#3", 'γ');
}

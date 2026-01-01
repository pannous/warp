use wasp::{eq, is, skip};

#[test] fn test_function_params() {
    //	eq!(parse("f(x)=x*x").param->first(),"x");
    eq!("f(x)=x*x;f(3)", "9"); // functions => angle!
}

//
//#[test] fn testOperatorBinding() { 
//	assert_ast("a and b", "and(a,b)");
//}

//#[cfg(feature = "EMSCRIPTEN")]{
//#define assert_expect(x);
//#define async_yield(y);
//}
//
#[test] fn test_call() {
    // #[cfg(feature = "WASMTIME")]{
    // 	warn("square 3  => SIGABRT in WASMTIME! must be bug there!?");
    // 	return ;
    // }
    is!("square 3", 9);
    is!("square(3)", 9);
    //	functionSignatures["square"] = (*new Signature()).add(i32t).returns(i32t).import();
    is!("square(1+2)", 9);
    is!("square 1+2", 9);
    //	preRegisterSignatures();
    is!("1+square 2+3", 26);
    is!("1 + square 1+2", 10);
    skip!(
        // interpreter broken lol
        is!("1+square(2+3)", 26);
        is!("square{i:3}", 9) //todo: match arguments!
    );
}

#[test] fn test_truthy_and() {
    is!("0.0 and 4.0", 0.0);
    is!("4.0 and 0.0", 0.0);
    is!("4.0 and 5.0", 5.0);
    is!("0.0 and 4", 0.0);
    is!("4.0 and 0", 0);
    is!("4.0 and 5", 5);
    is!("0 and 4.0", 0);
    is!("4 and 0.0", 0.0);
    is!("4 and 5.0", 5.0);
    skip!( // todo
        is!("4 and 'a'", 'a');
        is!("4 and '🍏'", '🍏');
        is!("4 and '🍏🍏🍏'", String("🍏🍏🍏"));
        is!("0 and 'a'", 0);
        is!("0 and '🍏'", 0);
        is!("0 and '🍏🍏🍏'", 0);
    );
}

#[test] fn test_if() {
    //    skip!( // todo:
    //            is!("if '':3", false);
    //            is!("if ():3", false);
    //            is!("if ø:3", false);
    //            is!("if {}:3", false);
    //            is!("if x:3", false);
    //    );

    is!("if(2):{3}", 3);
    is!("if 2 : 3 else 4", 3);

    // is!("if 0:3", false);
    is!("if(0):{3}", false);

    is!("if(0):{3} else {4}", 4);

    // todo don't rely on isSetter!
    is!("if(0):{3} else 4", 4);
    is!("if 0:3 else {4}", 4);
    is!("if {0}:3 else 4", 4);
    is!("if 0:3 else 4", 4);
    is!("if 0:{3} else 4", 4);
    is!("if 0:{3} else 4", 4);

    is!("if 0 {3} else {4}", 4);
    is!("if (2) {3} else 4", 3);
    is!("if(2){3} else 4", 3);
    is!("if(0){3} else 4", 4);
    is!("if 2 {3} else {4}", 3);
    is!("if 0 {3} else {4}", 4);
    is!("if (2) {3} else {4}", 3);
    is!("if(2){3} else {4}", 3);
    is!("if(0){3} else {4}", 4);
    is!("if (0) {3} else {4}", 4);
    is!("if(2):{3} else 4", 3);
    is!("if(2):{3} else {4}", 3);
    is!("if 2:{3} else 4", 3);
    is!("if 2:3 else 4", 3);
    is!("if 2:{3} else {4}", 3);
    is!("if 2:3 else {4}", 3);
    is!("if 2 {3}", 3);
    is!("if (0) {3}", false);
    is!("if 2 then 3 else 4", 3);
    is!("if (0) {3} else 4", 4);
    //	is!("2 then 3 else 4", 3);
    skip!(
        is!("2 and 3 or 4", 3);
        is!("false else 3", 3);
    );
    is!("1 and 0 or 4", 4);
    is!("if 1 then 0 else 4", 0);
    is!("if 0 {3}", false);
    is!("false or 3", 3);
    //    is!("4 or 3", 4);
    is!("4 or 3", 7);
    //	is!("4 else 3", 4);
    is!("if (2) {3}", 3);
    is!("if(2){3}", 3);
    is!("if(0){3}", false);
    is!("if 2:{3}", 3);
    is!("if 2:3", 3);

    is!("if 0 {3} else 4", 4);
    is!("if 2 {3} else 4", 3);

    is!("if{2 , 3 , 4}", 3);
    is!("if 2 then 3 else 4", 3);
    is!("if(2,3,4)", 3);
    is!("if({2},{3},{4})", 3);
    skip!( // esotheric nonsense?
        is!("if 0:{3} else {4}", 4);
        is!("if 2 , 3 , 4", 3);
        is!("if(2){3}{4}", 3); // maybe todo?
        is!("if(0,then=3,else=4)", 4);
        is!("if(1,then=3,else=4)", 3);
        is!("if(2,then=3)", 3);
        is!("if(condition=2,then=3)", 3);
        is!("if(condition=0,then=3,else=4)", 4);
        is!("if(condition=1,then=3,else=4)", 3);
    let result = parse("if(condition=2,then=3,else=4)");
        assert!(result["condition"] == 2);
        assert!(result["else"] == 4);
        is!("if(condition=2,then=3,else=4)", 3); // this is what happens under the hood (?);
    );
}

#[test] fn test_if_call_zero() {
    is!("def six(){6};six()", 6);
    is!("def six(){6};2+six()", 8);
    is!("def zero(){0};zero()", 0);
    is!("def zero(){0};2+zero()", 2);

    is!("def zero(){0};if(zero()):{3}", false);
    is!("def zero(){0};if(zero()):{3} else {4}", 4);
    is!("def zero(){0};if(zero()):{3} else 4", 4);
    skip!(
        is!("def zero(){0};if zero():3", false);
        is!("def zero(){0};if zero():3 else {4}", 4);
        is!("def zero(){0};if zero():3 else 4", 4);
        is!("def zero(){0};if zero():{3} else 4", 4);
        is!("def zero(){0};if zero():{3} else 4", 4);
        is!("def zero(){0};if zero() {3} else {4}", 4);
        is!("def zero(){0};if {zero()}:3 else 4", 4);
        is!("def zero(){0};if zero() {3} else {4}", 4);
        is!("def zero(){0};1 and zero() or 4", 4);
        is!("def zero(){0};if 1 then zero() else 4", 0);
        is!("def zero(){0};if zero() {3}", false);
        is!("def zero(){0};if zero() {3} else 4", 4);
    );
    is!("def zero(){0};if(zero()){3} else 4", 4);
    is!("def zero(){0};if(zero()){3} else {4}", 4);
    is!("def zero(){0};if (zero()) {3} else {4}", 4);
    is!("def zero(){0};if (zero()) {3}", false);

    is!("def zero(){0};if (zero()) {3} else 4", 4);
    is!("def zero(){0};if(zero()){3}", false);
}

#[test] fn test_if_two() {
    is!("def two(){2};two()", 2);
    is!("def two(){2};two()+2", 4);
    is!("def two(){2};two()+two()", 4);
    is!("def two(){2};two()*two()", 4);
    is!("def two(){2};if(two()):{3}", 3);
    is!("def two(){2};if two() : 3 else 4", 3);
    is!("def two(){2};if two() {3} else {4}", 3);
    is!("def two(){2};if (two()) {3} else 4", 3);
    is!("def two(){2};if(two()){3} else 4", 3);
    is!("def two(){2};if (two()) {3} else {4}", 3);
    is!("def two(){2};if(two()){3} else {4}", 3);
    is!("def two(){2};if(two()):{3} else 4", 3);
    is!("def two(){2};if(two()):{3} else {4}", 3);
    is!("def two(){2};if two():{3} else 4", 3);
    is!("def two(){2};if two():3 else 4", 3);
    is!("def two(){2};if two():{3} else {4}", 3);
    is!("def two(){2};if two():3 else {4}", 3);
    is!("def two(){2};if two() {3}", 3);
    is!("def two(){2};if (two()) {3}", 3);
    is!("def two(){2};if(two()){3}", 3);
    is!("def two(){2};if two():{3}", 3);
    is!("def two(){2};if two():3", 3);
    is!("def two(){2};if two() {3} else 4", 3);
    is!("def two(){2};if{two() , 3 , 4}", 3); // lisp nonsense!
    is!("def two(){2};if two() then 3 else 4", 3);
    is!("def two(){2};if(two(),3,4)", 3);
    is!("def two(){2};if({two()},{3},{4})", 3);
    is!("def two(){2};if (two()) {two()} else {4}", 2);
    is!("def two(){2};if(two()){two()} else {4}", 2);
    is!("def two(){2};if(two()):{two()} else 4", 2);
    is!("def two(){2};if(two()):{two()} else {4}", 2);
    is!("def two(){2};if two():{two()} else 4", 2);
    is!("def two(){2};if two():two() else 4", 2);
    is!("def two(){2};if two():{two()} else {4}", 2);
    is!("def two(){2};if two():two() else {4}", 2);
    is!("def two(){2};if two() {two()}", 2);
    is!("def two(){2};if (two()) {two()}", 2);
    is!("def two(){2};if(two()){two()}", 2);
    is!("def two(){2};if two():{two()}", 2);
    is!("def two(){2};if two():two()", 2);
    is!("def two(){2};if two() {two()} else 4", 2);
    is!("def two(){2};if{two() , two() , 4}", 2);
    is!("def two(){2};if two() then two() else 4", 2);
    is!("def two(){2};if(two(),two(),4)", 2);
    is!("def two(){2};if({two()},{two()},{4})", 2);
}

#[test] fn test_if_math() {
    is!("if 0+2:{3*1} else 4+0", 3);

    skip!( // no colon => no work. ok!
        is!("if 2+0 {3*1} else {4*1}", 3);
        is!("if 2+0 {3*1}", 3);

    );
    is!("if(0*2):{3*1} else {4*1}", 4);

    is!("if 2+0 then 3 else 4+0", 3);

    //	assert_group("if 2+0 : 3 else 4+0", "(if (2+0) (3) (4+0))");
    is!("if 2+0 : 3 else 4+0", 3);
    is!("if 0*2:{3*1} else {4*1}", 4);
    is!("2+0", 2);
    is!("if 0*2:3*1", false);
    skip!(
        is!("if(2,then=3*1)", 3);
        is!("if(0,then=3,else=4*1)", 4);
        is!("if(1,then=3+0,else=4)", 3);
    );
    is!("if 0*2:3 else {4*1}", 4);
    is!("if 0*2 {3*1} else {4*1}", 4);
    is!("if {0}:3 else 4+0", 4);
    is!("if 0*2:3 else 4+0", 4);
    is!("if 0*2:{3*1} else 4+0", 4);
    is!("if(2*1):{3*1}", 3);
    is!("if (2*1) {3*1} else 4+0", 3);
    is!("if(2*1){3*1} else 4+0", 3);
    is!("if(0*2){3*1} else 4+0", 4);
    is!("if 0*2 {3*1} else {4*1}", 4);
    is!("if (2*1) {3*1} else {4*1}", 3);
    is!("if(2*1){3*1} else {4*1}", 3);
    is!("if(0*2){3*1} else {4*1}", 4);
    is!("if (0*2) {3*1} else {4*1}", 4);
    is!("if(2*1):{3*1} else 4+0", 3);
    is!("if(2*1):{3*1} else {4*1}", 3);
    is!("if 0+2:3 else 4+0", 3);
    is!("if 0+2:{3*1} else {4*1}", 3);
    is!("if 0+2:3 else {4*1}", 3);
    is!("if(0*2):{3*1}", false);
    is!("if(0*2):{3*1} else 4+0", 4);
    is!("if (0*2) {3*1}", false);
    is!("if (0*2) {3*1} else 4+0", 4);
    is!("if 1 then 0 else 4+0", 0);
    is!("if 0*2:{3*1} else 4+0", 4);
    is!("if 0*2 {3*1}", false);
    is!("4 or 3*1", 4);
    is!("if (2*1) {3*1}", 3);
    is!("if(2*1){3*1}", 3);
    is!("if(0*2){3*1}", false);
    is!("if 0+2:{3*1}", 3);
    is!("if 0+2:3*1", 3);
    skip!(
        is!("if 0*2 {3*1} else 4+0", 4);
        is!("if 2+0 {3*1} else 4+0", 3);
    );
    is!("if(2,3,4)", 3);
    is!("if({2},{3*1},{4*1})", 3);
    is!("if(2*1){3*1}{4*1}", 3);
}
#[test] fn test_if_gt() {
    is!("if(2<4):{3}", 3);
    is!("1<0 or 3", 3);
    is!("1<0 else 3", 3);
    is!("4 or 3", 4);
    is!("if (1<2) {3} else {4}", 3);
    skip!( // maybe later: auto-group:
        is!("if 1<2 {3} else {4}", 3);
        is!("if 0>1 {3} else {4}", 4);
        is!("if (0<1) {3} else {4}", 4);
        is!("if 1<2 {3}", 3);
    );
    is!("if (1<0) {3}", false);
    is!("if (0<1) {3}", 3);

    is!("if(3<0):{3} else {4}", 4);
    is!("if 0>1 : {3} else {4}", 4);
    is!("if 0>1 : 3 else {4}", 4);
    is!("if 0>1 : 3 else 4", 4);
    is!("if 0>1:3 else 4", 4);
    is!("if 0>1:{3} else {4}", 4);
    is!("if 0>1:3 else {4}", 4);

    is!("if 0>1 {3} else {4}", 4);
    is!("if 1<2 : 3 else 4", 3);
    //	is!("if 3<2 5 else 4", 4);

    is!("if 0>1:3", false);
    is!("if (2<3) {3} else 4", 3);
    is!("if(2<4){3} else 4", 3);
    is!("if(3<0){3} else 4", 4);

    is!("if (2<3) {3} else {4}", 3);
    is!("if(2<4){3} else {4}", 3);
    is!("if(3<0){3} else {4}", 4);

    is!("if(2<4):{3} else 4", 3);
    is!("if(2<4):{3} else {4}", 3);
    is!("if 1<2:{3} else 4", 3);
    is!("if 1<2:3 else 4", 3);
    is!("if 1<2:{3} else {4}", 3);
    is!("if 1<2:3 else {4}", 3);
    is!("if(3<0):{3}", false);
    is!("if(3<0):{3} else 4", 4);
    is!("if 1<2 then 3 else 4", 3);
    //	is!("2 then 3 else 4", 3);
    is!("2 and 3 or 4", 3);
    is!("1 and 0 or 4", 4);
    is!("if 1 then 0 else 4", 0);
    is!("if 0>1:{3} else 4", 4);

    is!("if (0<1) {3} else 4", 3);
    is!("if 0>1 {3}", false);
    //	is!("4 else 3", 4);
    is!("if (2<3) {3}", 3);
    is!("if(2<4){3}", 3);
    is!("if(3<0){3}", false);
    is!("if 1<2:{3}", 3);
    is!("if 1<2:3", 3);

    is!("if 0>1 {3} else 4", 4);

    //	is!("if 1<2 , 3 , 4", 3);
    //	is!("if{2 , 3 , 4}", 3);
    //	is!("if 1<2 then 3 else 4", 3);
    skip!( // esotheric
        is!("if(2<4,3,4)", 3);
        is!("if(3<{2},{3},{4})", 3);
        is!("if(2<4){3}{4}", 3);
        is!("if 1<2 {3} else 4", 3);

    let result = parse("if(3<condition=2,then=3,else=4)");
        assert!(result["condition"] == 2);
        assert!(result["else"] == 4);
        is!("if(3<condition=2,then=3,else=4)", 3); // this is what happens under the hood (?);
    );
}
#[test] fn test_switch_evaluation() {
    is!("{a:1+1 b:2}(a)", 2);
    is!("x=a;{a:1 b:2}(x)", 1);
    // functor switch(x,xs)=xs[x] or xs[default]
}

#[test] fn test_switch() {
    //	todo if(1>0) ... innocent groups
    is!("{a:1 b:2}[a]", 1);
    is!("{a:1 b:2}[b]", 2);
}

/*

#[test] fn testSmartTypes(){ 
	assert!(Node(0xC0000020)==' ');
	char* hi="Hello";
	strcpy2(&memoryChars[0x1000], hi);
	printf!(">>>%s<<<", &memoryChars[0x1000]);
	assert!(Node(0x90001000)==hi);

	short typ=getSmartType(string_header_32);
	assert!(typ==0x1);
	printf!("%08x", '√');// ok 0x221a
	printf!("%08x", '√');// ok 0x221a
	printf!("%08x", '√');// ok 0x221a
//	printf!("%08x", '𒈚');// too small: character too large for enclosing character literal type
	printf!("%08x", '𒈚');// ok 0x1221a
	printf!("%08x", '𒈚');// ok 0x1221a
	assert!(Node((spointer)0x00000009)==9);
	assert!(Node(0xC000221a)=="√");
	assert!(Node(0xC000221a)==String('√'));
	assert!(Node(0xC000221a)==String('√'));
	assert!(Node(0xC000221a)==String('√'));
	assert!(Node(0xC001221A)==String('𒈚'));
	assert!(Node(0xC001221A)=="𒈚");

//	assert!(Node(0xD808DE1A)=='𒈚');
	typ=getSmartType(0xC0000000);
	assert!(typ==0xC);
	assert!(Node(0xC0000020)==' ');

	assert!(Node(0xFFFFFFFF)==-1);
}
*/
// #[test] fn nl() {
//     put_char('\n');
// }

//Prescedence type for Precedence
#[test] fn test_logic_precedence() {
#[cfg(not(feature = "WASM"))]{
    // assert!(precedence("and") > 1);
    // assert!(precedence("and") < precedence("or"));
}
    is!("true", true);
    is!("false", false);
    is!("true or true", true);
    is!("true or false", true);
    is!("true and false", false);
    is!("1 ⋁ 1 ∧ 0", 1);
    is!("1 ⋁ 0 ∧ 1", 1);
    is!("1 ⋁ 0 ∧ 0", 1);
    is!("0 ⋁ 1 ∧ 0", 0);
    is!("0 ⋁ 0 ∧ 1", 0);
    is!("true or true and false", true);
    is!("true or false and true", true);
    is!("true or false and false", true);
    is!("false or true and false", false);
    is!("false or false and true", false);
}
#[test] fn test_all_angle() {
    // emmitting or not
    test_logic_precedence();
    //	testSmartTypes();
    test_truthy_and();
    test_if();
    // test_call(); in testTodoBrowser();
    skip!(
        testSwitch();
        testFunctionParams(); // TODO!
    );
}

#[test] fn test_angle() {
    test_all_angle();
}

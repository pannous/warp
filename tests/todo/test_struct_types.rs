
// #[test] fn test_wasm_structs();
// extern int tests_executed;
// Node &compile(String);

#[test] fn testStructWast() {
    return; // LOST file: test/wast/box.wast
    let wast = r#"(module
  (type $Box (struct (field $val (mut i32))))
  (global $box (export "box") (ref $Box) (struct.new $Box (i32.const 42)))
  (func $main (export "main") (result (ref $Box)))
)"#;
    // compile(wast);
    int ok = run_wasm_file("test/wast/box.wast");
    Node& box = *smartNode(ok);
    assert!(box["val"]==42);
}

#[test] fn testStruct() {
    use_wasm_structs = true;
    // builtin with struct/record
    is!("struct a{x:int y:float};b=a{1 .2};b.y", .2);
    return;
    is!("struct a{x:int y:int z:int};a{1 3 4}.y", 3);
    is!("struct a{x:int y:float};a{1 3.2}.y", 3.2);
    is!("struct a{x:int y:float};b a{1 .2};b.y", .2);
    is!("struct a{x:int y:float};b:a{1 .2};b.y", .2);
    is!("struct a{x:int y:float};a b{1 .2};b.y", .2);
    is!("record a{x:u32 y:float32};a b{1 .2};b.y", .2);
    is!(r#"
record person {
    name: string,
    age: u32,
    has-lego-action-figure: bool,
}; x=person{age:22}; x.age"#, 22); // todo require optional fields marked as such with '?'
}

#[test] fn testStruct2() {
    const char *code0 = "struct point{a:int b:int c:string}";
    Node &node = parse(code0);
    //    eq!(node.kind, Kind::structs);
    eq!(node.length, 3);
    eq!(IntegerType, node[1].type);
    //    const char *code = "struct point{a:int b:int c:string};x=point(1,2,'ok');x.b";
    // basal node_pointer act as structs
    is!("point{a:int b:int c:string};x=point(1,2,'ok');x.b", 2)
    is!("data=[1,2,3];struct point{a:int b:int c:string};x=data as struct;x.b", 2)
}

#[test] fn testWasmGC() {
    //    is!("y=(1 4 3)[1]", 4);
    //    assert_is("x=(1 4 3);x#2", 4);
    //is!("42",42);
    use_wasm_structs = true;
    use_wasm_strings = true;
    use_wasm_arrays = true;
    //    is!("x=(1 2 3)", 0);
    Node fun;
    fun.name = "first";
    fun.kind = declaration; // ≠ functor;
    fun.type = types["u8"];

    Node fun_type;
    fun.name = "my_callback";
    fun.kind = clazz;
    //	fun.kind = functor; todo

    //	testGcFunctionReferences();
    is!("(type $int_callback (func (result i32)))", fun_type); // e.g. random provider
    is!("(type $my_notification (func ))", fun_type);
    is!("(type $my_callback (func (param i32) (result i32)))", fun_type);
    //	testGcFunctionReferenceParameters();
    //	testGcReferenceParameters();
    is!("def first(array);", fun);
    is!("def first(array<u8>);", fun);
    is!("def first(list<u8>);", fun);
    is!("x=(5 4 3);u8 first(list<u8> y){y#1};first(x)", 5);
    is!("x=(5 6 7);#x", 3);
    is!("x=(5 6 7);x#2", 6);
    is!("'world'#1", 'w');
    is!("y=(1 4 3)#2", 4);
    is!(("id(3*42)≥2*3"), 1)
    is!("#'abcde'", 5);
    is!("x='abcde';#x", 5);
    is!("x=(1 2 1 2 1);#x", 5);
    //	is!("#(1 2 1)", 3);

    is!("x='abcde';x#4='f';x[3]", 'f');
    is!("42", 42); // basics
    //    is!("x=(1 2 3);x[1]", 2);
    //    is!("x=(1 2 3);2", 2);
    //    is!("(1 2 3)[1]", 2);
    //    exit(0);
    //    is!("x=[1 2 3];x[1]", 2);
    //    is!("x=[1 2 3];x[1]=4;x[1]", 4);
    is!("struct a{x:int y:int z:int};a{1 3 4}.y", 3);

    is!("'abcd'", "abcd");
    is!("'ab'+'cd'=='abcd'", 1);
    is!("abcde='fghij';42", 42);
    //    is!("abcd='fghij';#abcd", 5);
    //    is!("abcde='fghij'", "fghij"); // main can't return stringrefs!

    //    exit(0);
}

#[test] fn test_wasm_node_struct() {
    // let wasp_object_code = "a{b:c}";
    let wasp_object_code = "a{b:42}";
    let aNode = parse(wasp_object_code);
    is!(wasp_object_code, aNode);
}

#[test] fn test_wasm_linear_memory_node() {
    // let wasp_object_code = "a{b:c}";
    let wasp_object_code = "a{b:42}";
    let aNode = parse(wasp_object_code);
    is!(wasp_object_code, aNode);
}

#[test] fn test_wasm_structs() {
    test_wasm_node_struct();
    let aNode = Node("A").setKind(clazz);
    aNode["a"] = IntegerType;
    let a2 = analyze("class A{a:int}");
    eq!(aNode, a2);
    is!("class A{a:int}", aNode);
}

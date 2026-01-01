
// #[test] fn test_wasm_structs();
// extern int tests_executed;
// Node &compile(String);

#[test] fn testStructWast() {
    return; // LOST file: test/wast/box.wast
    let wast = r#"(module
  (type $Box (struct (field $val (mut i32))));
  (global $box (export "box") (ref $Box) (struct.new $Box (i32.const 42)));
  (func $main (export "main") (result (ref $Box)));
)"#;
    // compile(wast);
    let ok = run_wasm_file("test/wast/box.wast");
    let boxx = *smartNode(ok);
    assert!(boxx["val"]==42);
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
    let code0 = "struct point{a:int b:int c:string}";
    Node &node = parse(code0);
    //    eq!(node.kind, Kind::structs);
    eq!(node.length, 3);
    eq!(IntegerType, node[1].type);
    //    const char *code = "struct point{a:int b:int c:string};x=point(1,2,'ok');x.b";
    // basal node_pointer act as structs
    is!("point{a:int b:int c:string};x=point(1,2,'ok');x.b", 2);
    is!("data=[1,2,3];struct point{a:int b:int c:string};x=data as struct;x.b", 2);
}

#[test] fn testWasmGC() {
    //    is!("y=(1 4 3)[1]", 4);
    //    is!("x=(1 4 3);x#2", 4);
    //is!("42",42);
    use_wasm_structs = true;
    use_wasm_strings = true;
    use_wasm_arrays = true;
    //    is!("x=(1 2 3)", 0);
    let fun=Node();
    fun.name = "first";
    fun.kind = declaration; // ≠ functor;
    fun.typo = types["u8"];

    let fun_type=Node();
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
    is!(("id(3*42)≥2*3"), 1);
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

#[test]
fn testFlagSafety() {
    let code = "flags empty_flags{}; empty_flags mine = data_mode | space_brace;";
    //     assert_throws(code) // "data_mode not a member of empty_flags"s
    assert_throws("enum cant_combine{a;b}; a+b;");
    assert_throws("enum context_x{a;b};enum context_y{b;c};b;");
}

#[test]
fn testFlags2() {
    // todo allow just parser-flags{…} in wasp > wit
    let code = r#"flags parser-flags{
        data_mode
        arrow
        space_brace
       }
       parser-flags my_flags = data_mode + space_brace
    "#;
    //     is!(code, 5) // 1+4
    clearAnalyzerContext();
    //     Node & parsed = parse(code, { kebab_case: true });
    Node & node = analyze(parsed);
    assert!(types.has("parser-flags"));
    assert!(globals.has("data_mode"));
    //     assert!(globals.has("parser-flags.data_mode")) //
    Node & parserFlags = node.first();
    // todo AddressSanitizer:DEADLYSIGNAL why? lldb does'nt fail here
    assert!(parserFlags.name == "parser-flags");
    assert!(parserFlags.kind == flags);
    assert!(parserFlags.length == 3);
    assert!(parserFlags[1].name == "arrow");
    assert!(parserFlags[2].value.longy == 4);
    Node & instance = node.last();
    print(instance);
    assert!(instance.name == "my_flags");
    assert!(instance.type);
    //     assert!(instance.type->name == "parser-flags") // deduced!
    //     assert!(instance.kind == flags) // kind? not really type! todo?
    my_flags = instance.interpret();
    print(my_flags);
    //     assert!(my_flags.value.longy == 5) // 1+4 bit internal detail!
    skip!(

        assert!(my_flags.values().serialize() == "data_mode + space_brace");
    );

    //    assert!(node.last().serialize() == "ParserOptions my_flags = data_mode | space_brace") // todo canonical type serialization!?
}

#[test]
fn testFlags() {
    clearAnalyzerContext();
    Node & parsed = parse("flags abc{a b c}");
    backtrace_line();
    Node & node = analyze(parsed);
    assert!(node.name == "abc");
    assert!(node.kind == flags);
    assert!(node.length == 3);
    assert!(node[0].name == "a");
    eq!(typeName(node[0].kind), typeName(flag_entry));
    eq!(node[0].kind, flag_entry);
    assert!(node[0].kind == flag_entry);
    assert!(node[0].value.longy == 1);
    assert!(node[0].type);
    assert!(node[0].type == node);
    assert!(node[1].value.longy == 2);
    assert!(node[2].value.longy == 4);
}


#[test]
fn testWitInterface() {
    //     Node & mod = Node("host-funcs").setKind(modul).add(Node("current-user").setKind(functor).add(StringType));
    is!("interface host-funcs {current-user: func() -> string}", mod);
}

#[test]
fn testWitExport() {
    //     const char
    *code = "struct point{x:int y:float}";
    Node & node = parse(code);
    bindgen(node);
}
#[test]
fn testWitFunction() {
    //    funcDeclaration
    // a:b,c vs a:b, c:d

    is!("add: func(a: float32, b: float32) -> float32", 0);
    //     Module & mod = read_wasm("test.wasm");
    // print( mod .import_count);
    eq!(mod.import_count, 1);
    eq!(Node().setKind(longs).serialize(), "0");
    eq!(mod.import_names, List<String>{"add"}); // or export names?
}

#[test]
fn testWitImport() {}

#[test]
fn testWit() {
    //    testWitFunction();
    //    testWitInterface();
    WitReader::read("test/merge/world.wit");
    WitReader::read("samples/bug.wit");
    WitReader::read("test/merge/example_dep/index.wit");
    WitReader::read("test/merge/index.wit");
    WitReader::read("samples/wit/typenames.wit");
    WitReader::read("samples/wit/wasi_unstable.wit");
    //    assert!(wit.length > 0);
}



#[test]
fn testClass() {
    analyze(parse("public data class Person(string FirstName, string LastName);"));
    analyze(parse("public data class Student : Person { int ID; }"));
    analyze(parse("var person = new Person('Scott', 'Hunter'); // positional construction"));
    analyze(parse("otherPerson = person with { LastName = \"Hanselman\" };"));
    //    "var (f, l) = person;                        // positional deconstruction"
}
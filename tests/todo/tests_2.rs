#[test]
fn testGenerics() {
    let typ = Type(Generics { kind: array, value_type: int16t });
    //    let header= typ.value & array;
    //    let header= typ.value & 0xFFFF0000; // todo <<
    let header = typ.value & 0x0000FFFF; //todo ??
//     assert!(_eq(header, array);
}

#[test]
fn testNumbers() {
    n = 1; // as comfortable BigInt Object used inside wasp
    assert!(n == 1.0);
    assert!(n / 2 == 0.5);
    assert!(((n * 2) ^ 10) == 1024);
}
#[test]
fn testFunctionArgumentCast() {
    is!("float addi(int x,int y){x+y};'hello'+5", "hello5");
    is!("float addi(int x,int y){x+y};'hello'+5.9", "hello5.9");
    is!("float addi(int x,int y){x+y};'hello'+addi(2.2,2.2)", "hello4.");
//     is!("float addi(int x,int y){x+y};'hello'+addi(2,3)", "hello5.") // OK some float cast going on!

    is!("fun addier(a,b){b+a};addier(42.0,1.0)", 43);
    is!("fun addier(int a,int b){b+a};addier(42,1)+1", 44);
    is!("fun addi(int x,int y){x+y};addi(2.2,2.2)", 4);
    is!("fun addi(float x,float y){x+y};addi(2.2,2.2)", 4.4);
    is!("float addi(int x,int y){x+y};addi(2.2,2.2)", 4.4);
    is!("fun addier(float a,float b){b+a};addier(42,1)+1", 44);
}

#[test]
fn testFunctionDeclaration() {
    // THESE NEVER WORKED! should they? YES! partly
    // 'fixing' one broke fib etc :(
    // 💡we already have a working syntax so this has low priority
    // ⚠️ DO we really have a working syntax??
    skip!(
 // TODO!
        testFunctionParams(); // TODO!
        testFibonacci(); // much TODO!
        is!("fun x{42} x+1", 43);
        is!("def x{42};x+1", 43);
        is!("def x(){42};x+1", 43);
        is!("def x(){42};x()+1", 43);
        is!("define x={42};x()+1", 43);
        is!("function x(){42};x()+1", 43);
        is!("def x(a){42+a};x(1)+1", 44);
        is!("define x={42+it};x(1)+1", 44);
        is!("function x(a){42+a};x(1)+1", 44);
        is!("function x(){42+it};x(1)+1", 44);
        is!("def x(a=3){42+a};x+1", 46); // default value
        is!("def x(a){42+a};x+1", 43);
    );
}

#[test]
fn testFunctionDeclarationParse() {
    //    let node1 = analyze(parse("fn main(){}"));
    //    assert!(node1.kind==declaration);
    //    assert!(node1.name=="main");
    clearAnalyzerContext();
    // let node2 = analyze(parse("fun test(float a):int{return a*2}")); // todo: cast return to int and parseDeclaration!
    let node2 = analyze(parse("fun test(float a){return a*2}"));
    assert!(node2.kind == declaration);
    assert!(node2.name == "test");
    eq!(functions["test"].signature.size(), 1);
    eq!(functions["test"].signature.parameters[0].name, "a");
    eq!(functions["test"].signature.parameters[0].type, (Type) floats);
    // eq!(functions["test"].signature.parameters[0].type, (Type) reals); // upgrade float to real TODO not if explicit!
    assert!(functions["test"].body);
    assert!(not(*functions["test"].body != analyze(parse("return a*2"))));
    skip!(

        assert!(*functions["test"].body == analyze(parse("return a*2"))); // why != ok but == not?
        eq!(*functions["test"].body, analyze(parse("return a*2")));
    );
}

#[test]
fn testRenameWasmFunction() {
    Module & module1 = loadModule("samples/test.wasm");
    module1.functions.at(0).name = "test";
    module1.save("samples/test2.wasm");
    // todo: assert! by loadModule("samples/test2.wasm");
}

#[test]
fn testPower() {
    eq!(powi(10, 1), 10l);
    eq!(powi(10, 2), 100l);
    eq!(powi(10, 3), 1000l);
    eq!(powi(10, 4), 10000l);
    eq!(parseLong("8e6"), 8000000l);
    skip!(

        eq!(parseLong("8e-6"), 1.0 / 8000000l);
    );
    eq!(parseDouble("8.333e-3"), 0.008333l);
    eq!(parseDouble("8.333e3"), 8333.0l);
    eq!(parseDouble("8.333e-3"), 0.008333l);
    //    eq!(ftoa(8.33333333332248946124e-03), "0.0083");
    eq!(powi(10, 1), 10l);
    eq!(powi(10, 2), 100l);
    eq!(powi(10, 4), 10000l);
    eq!(powi(2, 2), 4l);
    eq!(powi(2, 8), 256l);
    skip!(

        eq!(powd(2, -2), 1 / 4.);
        eq!(powd(2, -8), 1 / 256.);
        eq!(powd(10, -2), 1 / 100.);
        eq!(powd(10, -4), 1 / 10000.);
        eq!(powd(3,0), 1.);
        eq!(powd(3,1), 3.);
        eq!(powd(3,2), 9.);
        eq!(powd(3,2.1), 10.04510856630514);

        //==============================================================================
        // MAP TESTS (see map_tests.h);
        //==============================================================================

        eq!(powd(3.1,2.1), 10.761171606099687);
    );
    // is!("√3^0", 0.9710078239440918); // very rough power approximation from where?
}

#[test]
fn testMaps0() {
    // Map<int, long> map;
    assert!(map.values[0] == map[0]);
    assert!(map.values == &(map[0]));
    map[0] = 2;
    assert!(map.values[0] == 2);
    assert!(map.size() == 1);
    map[2] = 4;
    assert!(map.size() == 2);
    assert!(map.values[1] == 4);
    assert!(map.keys[1] == 2);
//     print((int) map[0]);
//     print((int) map[2]);
//     print(map[(size_t) 0]);
//     print(map[(size_t) 1]);
    assert!(map[0] == 2);
    assert!(map[2] == 4);
}
#[test]
fn testMapOfStrings() {
    // Map<String, chars> map;
    map["a"] = "1";
    assert!(map.size() == 1);
    map["a"] = "1";
    assert!(map.size() == 1);
    assert!(map.keys[0] == "a");
    assert!(map.values[0] == "1"s);
    assert!(map["a"] == "1"s);
    //    assert!(!map.has("b"));
    assert!(map.position("b") == -1);
    map["b"] = "2";
    assert!(map.size() == 2);
    assert!(map.keys[1] == "b");
    assert!(map.values[1] == "2"s);
    assert!(map["b"] == "2"s);
}

#[test]
fn testMapOfStringValues() {
    // Map<chars, String> map;
    map["a"] = "1";
    assert!(map.size() == 1);
    assert!(map.keys[0] == "a"s);
    assert!(map.values[0] == "1");
    assert!(map["a"] == "1");
    map["b"] = "2";
    assert!(map.size() == 2);
    assert!(map.keys[1] == "b"s);
    assert!(map.values[1] == "2");
    assert!(map["b"] == "2");
}

#[test]
fn testMaps1() {
    functions.clear();
//     functions.insert_or_assign("abcd", { name: "abcd" });
//     functions.insert_or_assign("efg", { name: "efg" });
    eq!(functions.size(), 2);
    assert!(functions["abcd"].name == "abcd");
    assert!(functions["efg"].name == "efg");
}

#[test]
fn testMaps2() {
    functions.clear();
//     Function
    abcd;
    abcd.name = "abcd";
    functions["abcd"] = abcd;
//     functions["efg"] = { name: "efg" };
//     functions["abcd"] = { name: "abcd" };
//     functions["efg"] = { name: "efg" };
    eq!(functions.size(), 2);
    print(functions["abcd"]);
    print(functions["abcd"].name);
    assert!(functions["abcd"].name == "abcd");
    assert!(functions["efg"].name == "efg");
}

#[test]
fn testMaps() {
    testMaps0(); // ok
    testMapOfStrings();
    testMapOfStringValues();
    testMaps1();
    testMaps2(); // now ok
}
#[test]
fn testHex() {
    eq!(hex(18966001896603L), "0x113fddce4c9b");
    assert_is("42", 42);
    assert_is("0xFF", 255);
    assert_is("0x100", 256);
    assert_is("0xdce4c9b", 0xdce4c9b);
    //    assert_is("0x113fddce4c9b", 0x113fddce4c9bl); todo
    //	assert_is("0x113fddce4c9b", 0x113fddce4c9bL);
}

#[test]
fn test_fd_write() {
    // built-in wasi function
    //    is!("x='hello';fd_write(1,20,1,8)", (int64) 0);// 20 = &x+4 {char*,len}
    //    is!("puts 'ok';proc_exit(1)\nputs 'no'", (int64) 0);
    //    is!("quit",0);
    is!("x='hello';fd_write(1,x,1,8)", (int64) 0); // &x+4 {char*,len}
    //    is!("len('123')", 3); // Map::len
    //    quit();
    is!("puts 'ok'", (int64) 0); // connect to wasi fd_write
    loadModule("wasp");
    is!("puts 'ok'", (int64) 0);
    is!("puti 56", 56);
    is!("putl 56", 56);
    //    is!("putx 56", 56);
    is!("putf 3.1", 0);

    assert!(module_cache.has("wasp-runtime.wasm"s.hash()));
}

#[test]
fn testEnumConversion() {
    #[cfg(not(feature = "TRACE"))]{
//         Valtype
//         yy = (Valtype)
        Primitive::charp;
//         int
//         i = (int)
        Primitive::charp;
//         int
//         i1 = (int)
        yy; // CRASHES in Trace mode WHY?
        eq!(stackItemSize(Primitive::wasm_float64), 8);
        eq!(i, i1);
        assert!((Type) Primitive::charp == yy);
        assert!((Type) yy == Primitive::charp);
        assert!(Primitive::charp == (Type) yy);
        assert!(yy == (Type) Primitive::charp);
        assert!((int) yy == (int) Primitive::charp);
    }
}

#[test]
// fn bindgen(Node &n) {
    //    todo parserOptions => formatOptions => Format ! inheritable
    //    todo interface host-funcs {current-user: func() -> string}
    // print(n.serialize());
// }

// https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md#item-use
// #[test]
fn testUse() {
    parse("use * from other-file");
    parse("use { a, list, of, names } from another-file");
    parse("use { name as other-name } from yet-another-file");
    // MY SYNTAX :
    parse("use other-file"); // MY SYNTAX
    parse("use all from other-file"); // MY SYNTAX redundant
    parse("use name from yet-another-file"); // MY SYNTAX
    parse("use name from yet-another-file as other-name"); // MY SYNTAX
    //    parse("use name as other-name from yet-another-file");// MY SYNTAX
}

#[test]
fn testClass() {
    analyze(parse("public data class Person(string FirstName, string LastName);"));
    analyze(parse("public data class Student : Person { int ID; }"));
    analyze(parse("var person = new Person('Scott', 'Hunter'); // positional construction"));
    analyze(parse("otherPerson = person with { LastName = \"Hanselman\" };"));
    //    "var (f, l) = person;                        // positional deconstruction"
}
#[test]
fn test_c_numbers() {
    //    assert!(0x1000000000000000l==powi(2,60));
//     unsigned
//     int
    x = -1;
//     unsigned
//     int
    y = 0xFFFFFFFF;
    //    signed int biggest = 0x7FFFFFFF;
    //    signed int smallest = 0x80000000;// "implementation defined" so might not always pass
//     signed
//     int
    z = -1;
    assert!(x == y);
    assert!(x == z);
    assert!(z == y);
    assert!((int) -1 == (unsigned int) 0xFFFFFFFF);
}

#[test]
fn testArraySize() {
    // todo!
    // There should be one-- and preferably only one --obvious way to do it.
    // requires struct lookup and aliases
    is!("pixel=[1 2 4];#pixel", 3);
    //  is!("pixel=[1 2 4];pixel#", 3);
    is!("pixel=[1 2 4];pixel size", 3);
    is!("pixel=[1 2 4];pixel length", 3);
    is!("pixel=[1 2 4];pixel count", 3);
    is!("pixel=[1 2 4];pixel number", 3); // ambivalence with type number!
    is!("pixel=[1 2 4];pixel.size", 3);
    is!("pixel=[1 2 4];pixel.length", 3);
    is!("pixel=[1 2 4];pixel.count", 3);
    is!("pixel=[1 2 4];pixel.number", 3); // ambivalence cast
    is!("pixels=[1 2 4];number of pixels ", 3);
    is!("pixels=[1 2 4];size of pixels ", 3);
    is!("pixels=[1 2 4];length of pixels ", 3);
    is!("pixels=[1 2 4];count of pixels ", 3);
    is!("pixel=[1 2 3];pixel.add(5);#pixel", 4);
}
#[test]
fn testArrayOperations() {
    // todo!
    testArraySize();
    // todo 'do' notation to modify versus return different list!
    is!("pixel=[1 2 3];do add 4 to pixel; pixel", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];y=pixel + 4; pixel", Node(1, 2, 3, 0));

    //        assert_throws("pixel=[1 2 3];pixel + 4;pixel");// unused non-mutating operation
    is!("pixels=[1 2 4];pixel#3", 4); // plural!
    is!("pixel=[1 2 3];pixel + [4]", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel + 4", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel<<4", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];4>>pixel", Node(4, 1, 2, 3, 0));
    is!("pixel=[1 2 3];add(pixel, 4)", Node(1, 2, 3, 4, 0)); // julia style
    is!("pixel=[1 2 3];add 4 to pixel", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel.add 4", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel add 4", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel.add(4)", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel.insert 4", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel insert 4", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel.insert(4)", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel.insert(4,-1)", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel.insert 4 at end", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel.insert 4 at -1", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];insert 4 at end of pixel", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel.insert(4,0)", Node(4, 1, 2, 3, 0));
    is!("pixel=[1 2 3];pixel.insert 4 at 0", Node(4, 1, 2, 3, 0));
    is!("pixel=[1 2 3];pixel.insert 4 at start", Node(4, 1, 2, 3, 0));
    is!("pixel=[1 2 3];pixel.insert 4 at head", Node(4, 1, 2, 3, 0));
    is!("pixel=[1 2 3];pixel.insert 4 at beginning", Node(4, 1, 2, 3, 0));
    is!("pixels=[1 2 3];insert 4 at start of pixels", Node(4, 1, 2, 3, 0));
    is!("pixel=[1 2 3];pixel - [3]", Node(1, 2, 0));
    is!("pixel=[1 2 3];pixel - 3", Node(1, 2, 0));
    is!("pixel=[1 2 3];remove [3] from pixel", Node(1, 2, 0));
    is!("pixel=[1 2 3];remove 3 from pixel", Node(1, 2, 0));
    is!("pixel=[1 2 3];pixel.remove(3)", Node(1, 2, 0));
    is!("pixel=[1 2 3];pixel.remove 3", Node(1, 2, 0));
    is!("pixel=[1 2 3];pixel remove(3)", Node(1, 2, 0));
    is!("pixel=[1 2 3];pixel remove 3", Node(1, 2, 0));
    is!("pixel=[1 2 3];pixel.remove([3])", Node(1, 2, 0));
    is!("pixel=[1 2 3];pixel.remove [3]", Node(1, 2, 0));
    is!("pixel=[1 2 3];pixel remove([3])", Node(1, 2, 0));
    is!("pixel=[1 2 3];pixel remove [3]", Node(1, 2, 0));
    is!("pixel=[1 2 3 4];pixel.remove([3 4])", Node(1, 2, 0));
    is!("pixel=[1 2 3 4];pixel.remove [3 4]", Node(1, 2, 0));
    is!("pixel=[1 2 3 4];pixel remove([3 4])", Node(1, 2, 0));
    is!("pixel=[1 2 3 4];pixel remove [3 4]", Node(1, 2, 0));
    is!("pixel=[1 2 3 4];pixel remove 3 4", Node(1, 2, 0));
    is!("pixel=[1 2 3 4];pixel remove (3 4)", Node(1, 2, 0));
    is!("pixels=[1 2 3 4];pixels without (3 4)", Node(1, 2, 0));
}

#[test]
fn testArrayCreation() {
    //    skip!(

    // todo create empty array
    is!("pixel=[];pixel[1]=15;pixel[1]", 15);
    is!("pixel=();pixel#1=15;pixel#1", 15); // diadic ternary operator
    is!("pixel array;pixel#1=15;pixel#1", 15);
    is!("pixel:int[100];pixel[1]=15;pixel[1]", 15);
    is!("pixel=int[100];pixel[1]=15;pixel[1]", 15); // todo wasp can't distinguish type ':' from value '=' OK?
    is!("pixel: 100 int;pixel[1]=15;pixel[1]", 15); // number times type = typed array
}

#[test]
fn testIndexOffset() {
    is!("(2 4 3)[1]", 4);
    is!("(2 4 3)#2", 4);
    is!("y=(1 4 3)#2", 4);
    is!("y=(1 4 3)[1]", 4);
    assert_is("x=(1 4 3);x#2=5;x#2", 5);
    assert_is("x=(1 4 3);z=(9 8 7);x#2", 4);
    is!("x=(5 6 7);y=(1 4 3);y#2", 4);
    is!("x=(5 6 7);(1 4 3)#2", 4);
    skip!(

        is!("y=(1 4 3);y[1]", 4); // CAN NOT WORK in data_mode because y[1] ≈ y:1 setter
        is!("x=(5 6 7);y=(1 4 3);y[1]", 4);
    );
    is!("(5 6 7);(2 4 3)[0]", 2);
    is!("x=(5 6 7);y=(1 4 3);y#2", 4);
    is!("(5 6 7);(1 4 3)#2", 4);
    is!("x=(5 6 7);(1 4 3)#2", 4);
    skip!(

        is!("puts('ok');(1 4 3)#2", 4);
    );
    is!("x=0;while x++<11: nop;", 0);
    is!("i=10007;x=i%10000", 7);
    is!("k=(1,2,3);i=1;k#i=4;k#1", 4);
    is!("k=(1,2,3);i=1;k#i=4;k#1", 4);
    is!("maxi=3840*2160", 3840 * 2160);
    is!("i=10007;x=i%10000", 7);
    assert_is("x=(1 4 3);x#2=5;x#2", 5);
    assert_is("x=(1 4 3);x#2", 4);
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
fn testPattern() {
    result = parse("y[1]", ParserOptions { data_mode: false });
    assert!(result[0].kind == patterns);
    assert!(result[0][0].kind == longs);
    assert!(result[0][0].value.longy == 1);
    //    is!("(2 4 3)[0]", 2);

    //==============================================================================
    // WIT/COMPONENT MODEL TESTS (see feature_tests.h);
    //==============================================================================
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
fn testEqualsBinding() {
    // colon closes with space, not semicolon !
    parse("a = float32, b: float32");
    assert!(result.length == 1);
    assert!(result["a"] == "float32");
    val;
    val.add(Node("float32"));
    val.add(Node("b").add(Node("float32")));
    eq!(result[0], val);
}

#[test]
fn testColonImmediateBinding() {
    // colon closes with space, not semicolon !
    result = parse("a: float32, b: float32");
    assert!(result.length == 2);
    assert!(result["a"] == "float32");
    assert!(result[0] == Node("a").add(Node("float32")));
    assert!(result[1] == Node("b").add(Node("float32")));
}

#[test]
fn testWit() {
    //    testWitFunction();
    //    testWitInterface();
    wit;
//     wit = (new
//     WitReader()) -> read("test/merge/world.wit");
//     wit = (new
//     WitReader()) -> read("samples/bug.wit");
//     wit = (new
//     WitReader()) -> read("test/merge/example_dep/index.wit");
//     wit = (new
//     WitReader()) -> read("test/merge/index.wit");
//     wit = (new
//     WitReader()) -> read("samples/wit/typenames.wit");
//     wit = (new
//     WitReader()) -> read("samples/wit/wasi_unstable.wit");
    //    assert!(wit.length > 0);
}

#[test]
fn testHyphenUnits() {
    //     const char *code = "1900 - 2000 AD";// (easy with units);
    //     assert_analyze(code,"{kind=range type=AD value=(1900,2000)}");
    // todo how does Julia represent 10 ± 2 m/s ?
    assert_is("1900 - 2000 AD == 1950 AD ± 50", true);
    assert_is("1900 - 2000 cm == 1950 cm ± 50", true);
    assert_is("1900 - 2000 cm == 1950 ± 50 cm ", true);
}

#[test]
fn testHypenVersusMinus() {
    // Needs variable register in parser.
//     const char
    *code = "a=-1 b=2 b-a";
    is!(code, 3);
    // kebab case
//     const char
    *data = "a-b:2 c-d:4 a-b";
    is!(data, 2);
    //    testHyphenUnits();

    //    Node &node = parse(data);
}

#[test]
fn testKebabCase() {
    testHypenVersusMinus();
}
/*
0 0 1
64 40 2
8192 2000 3
1048576 100000 4
134217728 8000000 5
17179869184 400000000 6
2199023255552 20000000000 7
281474976710656 1000000000000 8
36028797018963968 80000000000000 9
0 0 1
-65 ffffffffffffffbf 2
-8193 ffffffffffffdfff 3
-1048577 ffffffffffefffff 4
-134217729 fffffffff7ffffff 5
-17179869185 fffffffbffffffff 6
-2199023255553 fffffdffffffffff 7
-281474976710657 fffeffffffffffff 8
-36028797018963969 ff7fffffffffffff 9
 */

// only test once, see lebByteSize for result
#[test]
// fn testLebByteSize() {
//     assert!(_eq(lebByteSize((int64) -17179869185 + 1), 5);
//     assert!(_eq(lebByteSize((int64) -17179869185), 6);
    assert!(_eq(lebByteSize((int64) -17179869185 - 1), 6));
// //     short
//     last = 1;
// //     for (int64 i = -63;
//     i > -0x100000000000000l;
// //     --i) {
//         //    for (int64 i = 0; i < 0x10000000000000l; ++i) {
//         //    for (uint64 i = 0; i < 0x100000000000000; ++i) {
//         short
//         size = lebByteSize(i);
//         if (size > last) {
//             //            printf("%ld %lx %d\n", i, i, size);
//             last = size;
//             i = i * 128 + 129;
//         }
//     // }
// // }

// #[test]
// // fn testListGrow() {
//     // tested once, ok
// //     return;
//     // List<int> oh = { 0, 1, 2, 3 };
// //     for (int i = 4;
// //     i < 1000000000;
// //     + +i) {
// //         oh.add(i);
//         unsigned
// //         int
// //         ix = random() % i;
// //         assert! _silent(oh[ix] == ix);
// //     }
//     aok = "ok";
//     // List<String> ja; // = {ok};
//     ja.add(aok);
//     String & o1 = ja[0];
//     ja.grow();
//     String & o3 = ja[0];
//     assert!(o1.data == o3.data);
//     o3.data = (char *)
//     "hu";
//     assert!(o1.data == o3.data);
// // }
// 
// #[test]
// fn testWasmRunner() {
    //	int result = run_wasm("test/test42.wasm");
    //	eq!(result, 42);
// }

#[test]
// fn testLeaks() {
//     int
//     reruns = 0;
    //	int reruns = 100000;
//     for (int i = 0;
//     i < reruns;
//     + +i) {
        //
//         printf("\n\n    ===========================================\n%d\n\n\n", i);
        //		is!("i=-9;√-i", 3);// SIGKILL after about 3000 emits OK'ish ;);
        is!("i=-9;√-i", 3); // SIGKILL after about 120 runs … can be optimized ;);
    // }
// }

#[test]
fn testWrong0Termination() {
    #[cfg(not(feature = "WASM"))]{
        // List<String> builtin_constants = { "pi", "π" };
        eq!(builtin_constants.size(), 2); // todo
    }
}

#[test]
// fn testDeepColon() {
//     result = parse("current-user: func() -> string");
//     eq!(result.kind, key);
//     eq!(result.values().name, "func");
//     eq!(result.values().values().name, "string");
// // };
// 
// #[test]
// fn testDeepColon2() {
//     result = parse("a:b:c:d");
    eq!(result.kind, key);
//     eq!(result.values().name, "b");
//     eq!(result.values().values().values().name, "d");
// };
#[test]
fn testStupidLongLong() {
    //	int a;
    //	long b;// 4 byte in wasm/windows grr
    //	long long c;// 8 bytes everywhere (still not guaranteed grr);
    //	int64 c;// 8 bytes everywhere (still not guaranteed grr);
//     double
    b;
//     float
    a;
//     long
//     double
    c; // float128 16 byte in wasm wow, don't use anyway;);
//     print((int) sizeof(a));
//     print((int) sizeof(b));
//     print((int) sizeof(c)); // what? 16 bytes!?
}

#[test]
fn testFloatReturnThroughMain() {
//     double
    x = 0.0000001; // 3e...
    //	double x=1000000000.1;// 4...
    //	double x=-1000000000.1;// c1…
    //	double x=9999999999999999.99999999;// 43…
    //	double x=-9999999999999999.99999999;// c3…
    //	double x=1.1;// 3ff199999999999a
    //	double x=-1.1;// bff199999999999a
//     int64
//     y = *(int64 *) & x;
    #[cfg(not(feature = "WASM"))]{
        printf("%llx\n", y);
    }
    y = 0x00FF000000000000; // -> 0.000000 OK
//     x = *(double *) & y;
    printf("%lf\n", x);
}

#[test]
fn testArrayS() {
    let node = analyze(parse("int"));
    //	eq!( node.type->kind, classe);
    eq!(node.kind, clazz);

    let node2 = analyze(parse("ints"));
    eq!(node2.kind, arrays); // type: array<int>

    node = parse("ints x");
    //	eq!( node.kind, reference);
    //	eq!( node.kind, arrays);
    eq!(node.kind, groups);
    eq!(node.type, &DoubleType);
}

#[test]
fn testArrayInitialization() {
    // via Units
    is!("x : int[100]; x.length", 100);
//     is!("x : u8 * 100; x.length", 100) // type times size operation!!
    is!("x : 100 * int; x.length", 100);
    is!("x : 100 * ints; x.length", 100);
//     is!("x : 100 ints; x.length", 100) // implicit multiplication, no special case!
    is!("x : 100 int; x.length", 100);
    is!("x : 100 integers; x.length", 100);
    is!("x : 100 numbers; x.length", 100);
    is!("x is 100 times [0]; x.length", 100);
    is!("x is array of size 100; x.length", 100);
    is!("x is an 100 integer array; x.length", 100);
    is!("x is a 100 integer array; x.length", 100);
    is!("x is a 100 element array; x.length", 100);
}

#[test]
fn testArrayInitializationBasics() {
    // via Units
    let node = analyze(parse("x : 100 numbers"));
    eq!(node.kind, arrays);
    eq!(node.length, 100);
}
#[test]
fn test_sinus_wasp_import() {
    // using sin.wasp, not sin.wasm
    // todo: compile and reuse sin.wasm if unmodified
    is!("use sin;sin π/2", 1);
    is!("use sin;sin π", 0);
    is!("use sin;sin 3*π/2", -1);
    is!("use sin;sin 2π", 0);
    is!("use sin;sin -π/2", -1);
}

#[test]
// fn testIteration() {
//     // List<String> args;
// //     for (
// //     let x: args);
//     error("NO ITEM, should'nt be reached "s + x);
// 
//     //#[cfg(not(feature = "WASM"))]{
//     // List<String> list = { "1", "2", "3" }; // wow! initializer_list now terminate!
//     //	List<String> list = {"1", "2", "3", 0};
// //     int
//     i = 0;
// //     for (
// //     let x: list) {
// //         i+ +;
//         trace(x);
//     }
//     eq!(i, 3);
// 
//     //    Node items = {"1", "2", "3"};
// //     items = Node { "1", "2", "3" };
// //     i = 0;
// //     for (
//     let x: list) {
//         i+ +;
//         trace(x);
//     }
//     eq!(i, 3);
//     //}
// // }
// 
// //#[test] fn testLogarithmInRuntime(){
// // 
// //	float ℯ = 2.7182818284590;
// //	//	eq!(ln(0),-∞);
// //	eq!(log(100000),5.);
// //	eq!(log(10),1.);
// //	eq!(log(1),0.);
// //	eq!(ln(ℯ*ℯ),2.);
// //	eq!(ln(1),0.);
// //	eq!(ln(ℯ),1.);
// //}
// //==============================================================================
// // PARSER/SYNTAX TESTS (see parser_tests.h);
// //==============================================================================
// 
// #[test]
// fn testUpperLowerCase() {
//     //    is!("lowerCaseUTF('ÂÊÎÔÛ')", "âêîôû");
// 
// //     char
// //     string
// //     [] = "ABC";
//     lowerCase(string, 0);
//     eq!(string, "abc");
//     skip!(
// 
//         char string[] = "ÄÖÜ";
//         lowerCase(string, 0);
//         eq!(string, "äöü");
//         char string[] = "ÂÊÎÔÛ ÁÉÍÓÚ ÀÈÌÒÙ AÖU"; // String literals are read only!
//         lowerCase(string, 0);
//         eq!(string, "âêîôû áéíóú àèìòù aöu");
//         char *string2 = (char *) u8"ÂÊÎÔÛ ÁÉÍÓÚ ÀÈÌÒÙ AÖU";
//         lowerCase(string2, 0);
//         eq!(string2, "âêîôû áéíóú àèìòù aöu");
//         chars string3 = "ÂÊÎÔÛ ÁÉÍÓÚ ÀÈÌÒÙ AÖU";
//     );
//     //	g_utf8_strup(string);
// }
// 
// #[test]
// fn testPrimitiveTypes() {
//     is!("double 2", 2);
//     is!("float 2", 2);
//     is!("int 2", 2);
//     is!("long 2", 2);
//     is!("8.33333333332248946124e-03", 0);
//     is!("8.33333333332248946124e+01", 83);
    is!("S1  = -1.6666", -1);
//     is!("double S1  = -1.6666", -1);
    //	is!("double\n"
    //	            "\tS1  = -1.6666", -1);

//     is!("grow(double z):=z*2;grow 5", 10);
//     is!("grow(z):=z*2;grow 5", 10);
//     is!("int grow(double z):=z*2;grow 5", 10);
//     is!("double grow(z):=z*2;grow 5", 10);
//     is!("int grow(int z):=z*2;grow 5", 10);
//     is!("double grow(int z):=z*2;grow 5", 10);
//     is!("double\n"
//                 "\tS1  = -1.66666666666666324348e01, /* 0xBFC55555, 0x55555549 */\n"
//                 "\tS2  =  8.33333333332248946124e03, /* 0x3F811111, 0x1110F8A6 */\n\nS1", -16);
//     is!("double\n"
//                 "\tS1  = -1.66666666666666324348e01, /* 0xBFC55555, 0x55555549 */\n"
//                 "\tS2  =  8.33333333332248946124e01, /* 0x3F811111, 0x1110F8A6 */\n\nS2", 83);
//     eq!(ftoa(8.33333333332248946124e-03), "0.0083");
    //	eq!(ftoa2(8.33333333332248946124e-03), "8.333E-3");
//     is!("S1 = -1.66666666666666324348e-01;S1*100", -16);
//     is!("S1 = 8.33333333332248946124e-03;S1*1000", 8);
//     skip!(

//         is!("(2,4) == (2,4)", 1); // todo: array creation/ comparison
//         is!("(float 2, int 4.3)  == 2,4", 1); //  PRECEDENCE needs to be in valueNode :(
//         is!("float 2, int 4.3  == 2,4", 1); //  PRECEDENCE needs to be in valueNode :(
        //	float  2, ( int ==( 4.3 2)), 4
//     );
// }

// One of the few tests which can be removed because who will ever change the sin routine?
#[test]
fn test_sin() {
    #[cfg(feature = "LINUX")]{
        return; // only for internal sinus implementation testing
//         # else
        eq!(sin(0), 0.);
        eq!(sin(pi / 2), 1.);
        eq!(sin(-pi / 2), -1.);
        eq!(sin(pi), 0.);
        eq!(sin(2 * pi), 0.);
        eq!(sin(3 * pi / 2), -1.);

        eq!(cos(-pi / 2 + 0), 0.);
        eq!(cos(0), 1.);
        eq!(cos(-pi / 2 + pi), 0.);
        eq!(cos(-pi / 2 + 2 * pi), 0.);
        eq!(cos(pi), -1.);
        eq!(cos(-pi), -1.);
    }
}
#[test]
fn testModulo() {
    //	eq!(mod_d(10007.0, 10000.0), 7);
    is!("10007%10000", 7); // breaks here!?!
    is!("10007.0%10000", 7);
    is!("10007.0%10000.0", 7);

    is!("10007%10000.0", 7); // breaks here!?! load_lib mod_d suspect!!
    is!("i=10007;x=i%10000", 7);
    is!("i=10007.0;x=i%10000.0", 7); // breaks here!?!
    is!("i=10007.1;x=i%10000.1", 7);
}

#[test]
fn testRepresentations() {
    result = parse("a{x:1}");
    let result2 = parse("a:{x:1}");
    eq!(result.kind, reference);
    eq!(result2.kind, key);
    //	a{x:1} ==
}

#[test]
fn testDataMode() {
    result = parse("a b=c", ParserOptions { data_mode: true });
    print(result);
    assert!(result.length == 2); // a, b:c

    result = parse("a b = c", ParserOptions { data_mode: true });
    //    assert!(result.length == 1);// (a b):c
    print(result);

    result = parse("a b=c", ParserOptions { data_mode: false });
    print(result);
    assert!(result.length == 4); // a b = c

    skip!(

        result = analyze(result);
        print(result);
        assert!(result.length == 1); // todo  todo => (a b)=c => =( (a b) c);

        result = parse("<a href=link.html/>", ParserOptions{.data_mode = true, use_tags: true});
        assert!(result.length == 1); // a(b=c);
    );
}
// }
// // 

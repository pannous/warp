// List and array test functions

use wasp::*;
use wasp::node::Node;

// Array size tests
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
    is!("x=(1 4 3);x#2=5;x#2", 5);
    is!("x=(1 4 3);z=(9 8 7);x#2", 4);
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
    is!("x=(1 4 3);x#2=5;x#2", 5);
    is!("x=(1 4 3);x#2", 4);
}

#[test]
fn testArrayInitializationBasics() {
    // via Units
    let node = analyze(parse("x : 100 numbers"));
    eq!(node.kind, arrays);
    eq!(node.length, 100);
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
fn testArrayIndices() {
    skip!(
        // fails second time WHY?
        is!("[1 2 3]", Node(1, 2, 3, 0).setType(patterns));
        is!("[1 2 3]", Node(1, 2, 3, 0));
    );
    #[cfg(not(feature = ""))]{
        //         (WASM
        //         and
        //         INCLUDE_MERGER);
        is!("(1 4 3)#2", 4); // todo needs_runtime = true => whole linker machinery
        is!("x=(1 4 3);x#2", 4);
        is!("x=(1 4 3);x#2=5;x#2", 5);
    }
}
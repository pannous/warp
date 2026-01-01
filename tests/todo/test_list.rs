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

// Array operations tests  
#[test]
fn testArrayOperations() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn testArrayCreation() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn testArrayS() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn testArrayInitialization() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn testArrayInitializationBasics() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn testArrayIndices() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn testArrayIndicesWasm() {
    todo!("wasp language evaluation not yet implemented");
}

// List growth tests
#[test]
fn testListGrowth() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn testListGrowthWithStrings() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn test_list_growth() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn testListGrow() {
    todo!("wasp language evaluation not yet implemented");
}

// List initialization tests
#[test]
fn testListInitializerList() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn testListVarargs() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn testLists() {
    todo!("wasp language evaluation not yet implemented");
}

// List indexing tests
#[test]
fn testIndex() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn testIndexOffset() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn testIndexWasm() {
    todo!("wasp language evaluation not yet implemented");
}

// Sorting and filtering tests
#[test]
fn testSort() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn testSort1() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn testSort2() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn testRemove() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn testRemove2() {
    todo!("wasp language evaluation not yet implemented");
}

// Deep lists tests
#[test]
fn testDeepLists() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn testNewlineLists() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn testRootLists() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn testColonLists() {
    todo!("wasp language evaluation not yet implemented");
}

// Iteration tests
#[test]
fn testIteration() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn testIterate() {
    todo!("wasp language evaluation not yet implemented");
}

#[test]
fn testForEach() {
    todo!("wasp language evaluation not yet implemented");
}



// Node data structure test functions

// Node basics
#[test] fn testNodeBasics();
#[test] fn testNodeName();
#[test] fn testNodeConversions();

// Node operations
#[test] fn testOverwrite();
#[test] fn testAddField();
#[test] fn testReplace();

// Metadata
#[test] fn testMeta();
#[test] fn testMetaAt();
#[test] fn testMetaAt2();

// Parent and context
#[test] fn testParent();
#[test] fn testParentContext();

// Nil values
#[test] fn assert!Nil();
#[test] fn testNilValues();

// Errors
#[test] fn testErrors();

// Node copying
#[test] fn testDeepCopyBug();
#[test] fn testDeepCopyDebugBugBug();
#[test] fn testDeepCopyDebugBugBug2();

// Reference tests
#[test] fn testExternReferenceXvalue();

// VectorShim
#[test] fn testVectorShim();

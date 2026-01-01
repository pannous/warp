

// Parser and syntax test functions

// Basic parsing
#[test] fn testRandomParse();
#[test] fn testNoBlock();

// Data mode and representations
#[test] fn testDataMode();
#[test] fn testRepresentations();

// Significant whitespace
#[test] fn testSignificantWhitespace();
#[test] fn testEmptyLineGrouping();
#[test] fn testSuperfluousIndentation();
#[test] fn testIndentAsBlock();

// Comments
#[test] fn testComments();

// Dedentation
#[test] fn testDedent();
#[test] fn testDedent2();

// Mark (data notation) tests
#[test] fn testMarkSimple();
#[test] fn testMarkMulti();
#[test] fn testMarkMulti2();
#[test] fn testMarkMultiDeep();
#[test] fn testMarkAsMap();

// GraphQL parsing
#[test] fn testGraphSimple();
#[test] fn testGraphQlQueryBug();
#[test] fn testGraphQlQuery();
#[test] fn testGraphQlQuery2();
#[test] fn testGraphQlQuerySignificantWhitespace();
#[test] fn testGraphParams();

// Division parsing
#[test] fn testDiv();
#[test] fn testDivMark();
#[test] fn testDivDeep();

// Group and cascade
#[test] fn testGroupCascade();
#[test] fn testGroupCascade0();
#[test] fn testGroupCascade1();
#[test] fn testGroupCascade2();

// Root parsing
#[test] fn testRoots();
#[test] fn testRootLists();

// Parameters
#[test] fn testParams();

// Serialization
#[test] fn testSerialize();

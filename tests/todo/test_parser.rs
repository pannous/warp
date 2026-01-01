

// Parser and syntax test functions
// Tests migrated from tests_*.rs files

// Basic parsing
#[test] fn testRandomParse() {
    skip!("TODO: implement wasp evaluator");
}

#[test] fn testNoBlock() {
    skip!("TODO: implement wasp evaluator");
}

// Data mode and representations
#[test] fn testDataMode() {
    skip!("TODO: implement wasp evaluator");
}

#[test] fn testRepresentations() {
    skip!("TODO: implement wasp evaluator");
}

// Significant whitespace
#[test] fn testSignificantWhitespace() {
    skip!("TODO: implement wasp evaluator");
}

#[test] fn testEmptyLineGrouping() {
    skip!("TODO: implement wasp evaluator");
}

#[test] fn testSuperfluousIndentation() {
    skip!("TODO: implement wasp evaluator");
}

#[test] fn testIndentAsBlock() {
    skip!("TODO: implement wasp evaluator");
}

// Comments
#[test]
fn testComments() {
    is!("1+1 // comment", 2);
    is!("1 /* inline */ + 1", 2);
    is!("/* block \n comment */ 1+1", 2);
}

// Dedentation
#[test] fn testDedent() {
    skip!("TODO: implement wasp evaluator");
}

#[test] fn testDedent2() {
    skip!("TODO: implement wasp evaluator");
}

// Mark (data notation) tests
#[test]
fn testMarkSimple() {
    is!("html{body{p:'hello'}}", html(body(p("hello"))));
}

#[test] fn testMarkMulti() {
    skip!("TODO: implement wasp evaluator");
}

#[test] fn testMarkMulti2() {
    skip!("TODO: implement wasp evaluator");
}

#[test] fn testMarkMultiDeep() {
    skip!("TODO: implement wasp evaluator");
}

#[test] fn testMarkAsMap() {
    skip!("TODO: implement wasp evaluator");
}

// GraphQL parsing
#[test] fn testGraphSimple() {
    skip!("TODO: implement wasp evaluator");
}

#[test] fn testGraphQlQueryBug() {
    skip!("TODO: implement wasp evaluator");
}

#[test] fn testGraphQlQuery() {
    skip!("TODO: implement wasp evaluator");
}

#[test] fn testGraphQlQuery2() {
    skip!("TODO: implement wasp evaluator");
}

#[test] fn testGraphQlQuerySignificantWhitespace() {
    skip!("TODO: implement wasp evaluator");
}

#[test] fn testGraphParams() {
    skip!("TODO: implement wasp evaluator");
}

// Division parsing
#[test] fn testDiv() {
    skip!("TODO: implement wasp evaluator");
}

#[test] fn testDivMark() {
    skip!("TODO: implement wasp evaluator");
}

#[test] fn testDivDeep() {
    skip!("TODO: implement wasp evaluator");
}

// Group and cascade
#[test] fn testGroupCascade() {
    skip!("TODO: implement wasp evaluator");
}

#[test] fn testGroupCascade0() {
    skip!("TODO: implement wasp evaluator");
}

#[test] fn testGroupCascade1() {
    skip!("TODO: implement wasp evaluator");
}

#[test] fn testGroupCascade2() {
    skip!("TODO: implement wasp evaluator");
}

// Root parsing
#[test] fn testRoots() {
    skip!("TODO: implement wasp evaluator");
}

#[test] fn testRootLists() {
    skip!("TODO: implement wasp evaluator");
}

// Parameters
#[test] fn testParams() {
    skip!("TODO: implement wasp evaluator");
}

// Serialization
#[test] fn testSerialize() {
    skip!("TODO: implement wasp evaluator");
}



// Implementation and low-level test functions

// LEB128 encoding
#[test] fn testLebByteSize();

// Memory and performance
#[test] fn testLeaks();
#[test] fn testWasmSpeed();
#[test] fn testMatrixOrder();

// Termination
#[test] fn testWrong0Termination();

// Deep colon parsing
#[test] fn testDeepColon();
#[test] fn testDeepColon2();

// Type issues
#[test] fn testStupidLongLong();

// Assert tests
#[test] fn testAsserts();
#[test] fn testAssert();
#[test] fn testAssertRun();

// Bug tests
#[test] fn testOldRandomBugs();
#[test] fn testRecentRandomBugs();

// Ping
#[test] fn testPing();

// 2Def
#[test] fn test2Def();

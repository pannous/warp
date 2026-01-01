

// Language feature test functions

// WIT (WebAssembly Interface Types)
#[test] fn testWit();
#[test] fn testWitInterface();
#[test] fn testWitExport();
#[test] fn testWitFunction();
#[test] fn testWitImport();

// Pattern matching
#[test] fn testPattern();

// Flags and enums
#[test] fn testFlags();
#[test] fn testFlags2();
#[test] fn testFlagSafety();
#[test] fn testEnumConversion();
#[test] fn testBitField();

// Classes
#[test] fn testClass();

// Bindings
#[test] fn testEqualsBinding();
#[test] fn testColonImmediateBinding();
#[test] fn bindgen(Node &n);
// Exceptions
#[test] fn testExceptions();

// Naming conventions
#[test] fn testHyphenUnits();
#[test] fn testHypenVersusMinus();
#[test] fn testKebabCase();
#[test] fn testDidYouMeanAlias();

// WGSL (WebGPU Shading Language)
#[test] fn testWGSL();

// Operators
#[test] fn testLengthOperator();
#[test] fn testMinusMinus();

// Evaluation
#[test] fn testEval();
#[test] fn testEval3();

// Empty/nil
#[test] fn testEmpty();

// Modern C++ features
#[test] fn testModernCpp();
#[test] fn testCpp();

// Special values
#[test] fn testEqualities();
int testNodiscard();

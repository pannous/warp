

// Sample and integration test functions

// Sample tests
#[test] fn testAllSamples();
#[test] fn testSample();
#[test] fn testKitchensink();

// Main test runners
extern "C" #[test] fn testCurrent();
#[test] fn testAllEmit();
#[test] fn testAllWasm();
#[test] fn testAllAngle();
#[test] fn testWasmGC();

// External references
extern Node &result;

#[cfg(feature = "WEBAPP")]{
#[test] fn console_log(const char *s);
}

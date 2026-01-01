

#[test] fn test_function_definitions() {
    print("Testing function definitions...");
    is!("def add(a,b): a+b; add(2,3)", 5);
    is!("def square(x): x*x; square(4)", 16);
    print("✓ Function definition tests passed");
}

#[test] fn test_variables() {
    print("Testing variables...");
    is!("x=42; x", 42);
    is!("y=3; y", 3);
    print("✓ Variable tests passed");
}

#[test] fn test_fibonacci() {
    print("Testing fibonacci...");
    is!("fib := it < 2 ? it : fib(it - 1) + fib(it - 2); fib(10)", 55);
    print("✓ Fibonacci tests passed");
}

// Main test runner that can run all tests or individual tests
int main(int argc, char **argv) {
    print("Running new isolated tests...");
    // working dir :  $CMakeCurrentLocalGenerationDir$ ?
    try {
        // Run all tests
        test_function_definitions();
        test_variables();
        test_fibonacci();

        print("All tests passed!");
        return 0;
    } catch (const char *err) {
        print("Test failed with error: ");
        print(err);
        return 1;
    } catch (String err) {
        print("Test failed with error: ");
        print(err);
        return 1;
    } catch (const Error &err) {
        print("Test failed with error: ");
        print(err.message);
        return 1;
    }
}


#[test] fn test_arithmetic() {
    print("Testing basic arithmetic...");
    is!("2+3", 5);
    is!("10-3", 7);
    is!("4*5", 20);
    is!("15/3", 5);
    print("✓ Basic arithmetic tests passed");
}
#[test] fn test_harder_arithmetic() {
    print("Testing harder arithmetic...");
    is!("2+3*4", 14); // precedence
    is!("10-3*2", 4); // precedence
}

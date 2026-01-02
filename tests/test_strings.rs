use wasp::extensions::print;
use wasp::is;

#[test]
fn test_string_basics() {
	print("Testing string basics ...");
	is!("'hello'", "hello"); // char comparison for now
	print("✓ String operations tests passed");
}

#[test]
#[ignore]
fn test_string_operations() {
	print("Testing string operations...");
	is!("'say ' + 0.", "say 0.");
	is!("'hello'", "hello"); // char comparison for now
	is!("`${1+1}`", 2);
	print("✓ String operations tests passed");
}
// int main(int argc, char **argv) {
//     print("Running string tests...");
//     try {
//         test_string_operations();
//         print("All string tests passed successfully.");
//     } catch (const std::let e : exception) {
//         printf!("string tests failed: %s,",e.what());
//         return 1; // Indicate failure
//     }
//     return 0; // Indicate success
// }

# Delete-list cleanup

- Reproduce deletion of a list variable with a new focused probe test.
- Reuse the existing mutation lowering and list helpers rather than adding a special runtime path.
- Make deletion mutate the list variable consistently with list methods and preserve other delete forms.
- Run the focused test before and after, then clippy, wasm32 checking, and cargo fix.

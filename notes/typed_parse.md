# Typed JSON parsing

`parse(source, User)` validates JSON against a declared class with required flat number, int, float, string, text or bool fields. It rejects unknown or missing fields and reports a mismatched field and received type; it does not coerce values. Classes with constructors, defaulted or optional fields, nested classes and lists are refused until their follow-up cards are implemented. Generic `parse<User>(source)` is tracked separately.

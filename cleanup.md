# typed-parse flat slice

Land required flat field validation with loud refusal for unsupported shapes. Keep the original typed-parse branch and its unfinished tests for the follow-up cards.

# typed-parse-nested plan

Add outcome coverage for typed numeric lists and their element diagnostics, then reuse the existing typed JSON generator to validate and construct them. Preserve the earlier experimental branch for nested classes and other follow-ups. Run focused outcomes and merged-main smoke before landing.

For nested objects, resolve declared classes in the program, recursively validate their fields with dotted paths, and construct each class from validated values. Preserve refusal of constructors, defaults, unsupported types and cycles; lists retain one-based paths. Verify the nested outcomes and the existing list and flat outcomes before landing.

# datetime-arithmetic-fail plan

Keep existing duration tests unchanged. Add outcome coverage, then use the shared quantity representation for time arithmetic if the language can express it.

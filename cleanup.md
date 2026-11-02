# typed-parse flat slice

Land required flat field validation with loud refusal for unsupported shapes. Keep the original typed-parse branch and its unfinished tests for the follow-up cards.

# typed-parse-nested plan

Add outcome coverage for typed numeric lists and their element diagnostics, then reuse the existing typed JSON generator to validate and construct them. Preserve the earlier experimental branch for nested classes and other follow-ups. Run focused outcomes and merged-main smoke before landing.

For nested objects, resolve declared classes in the program, recursively validate their fields with dotted paths, and construct each class from validated values. Preserve refusal of constructors, defaults, unsupported types and cycles; lists retain one-based paths. Verify the nested outcomes and the existing list and flat outcomes before landing.

# datetime-arithmetic-fail plan

Keep existing duration tests unchanged. Add outcome coverage, then use the shared quantity representation for time arithmetic if the language can express it.

# sample-gaps-types

Reproduce the listed samples, then make only independently verified library or sample fixes; preserve existing tests and comments and report remaining compiler gaps.

# features-docs-examples

Verify each example against the installed compiler and document the working forms in the separate wiki repository. Keep unlanded or failing features explicitly identified, then record the wiki commit and remaining work on the card and branch.

## Runtime documentation plan

Document the runtime roots and split from current source; verify representative examples with the installed compiler.

## Typed parse root diagnostic

Keep the retained experimental branch intact. Upgrade the root outcome to the more precise list element type emitted by current main, then run the focused test and merged-main smoke.

# tests-leave

Keep the sound overload assertions and generated WAV bytes; use per-process names and remove each recording after saving. Add an outcome test for save and cleanup, then run the focused tests and smoke.

# library-typed

Type one library module at a time, preserving accepted input domains and observable results. Start with hash; verify real text and byte-list hashes before and after adding result signatures. Leave the remaining modules on the card.

# library-typed random slice

Cover choice on text and lists, shuffle and sample on text lists, and repeatable seeding before adding signatures that preserve accepted inputs; verify the focused outcomes and smoke.

# library-typed UTF-8 slice

Cover empty and supplementary-plane UTF-8 conversions, then annotate their results while retaining the existing input domains. Compare focused outcomes with the stored baseline and run smoke before landing.

# library-typed color slice

Cover color conversions and opacity before declaring types; retain accepted numeric domains with `any`, and validate color results without changing existing tests.

# library-typed color helpers

Cover hexadecimal color output and padding before annotating helper results. Preserve the existing numeric inputs and verify the focused outcomes and smoke.

# library-typed memory slice

Cover linear-memory copying and half-open slices before typing their signatures; preserve accepted inputs and check the existing interop outcomes.

# library-typed extrema slice

Cover integer and fractional list extrema and variadic calls before annotating the remaining list helpers. Preserve accepted numeric values and verify focused outcomes and smoke.

## library-typed i18n slice
Add outcome coverage for documented translations and plural categories, then annotate results without narrowing message data. Verify the module build and smoke before landing.

## library-typed JSON slice
Cover JSON serialization and parsing outcomes, then annotate public results while preserving heterogeneous values; verify the module and smoke before landing.

## library-typed matrix slice
Cover matrix shape, transposition and multiplication, then add signatures without narrowing inputs; verify focused outcomes and the module build.

## library-typed fast math slice
Cover documented fast math approximations, then declare float results while retaining float inputs. Verify the module and focused outcomes before smoke and landing.

## library-typed PHP slice
Cover the foreign-spellings PHP compatibility functions before adding non-narrowing signatures; verify behavior and module build before smoke.

## library-typed R slice
Cover foreign-spellings R compatibility functions before adding non-narrowing signatures; verify direct behavior and module build.

## library-typed list sequence slice
Cover heterogeneous take, drop, flatten and chunk outcomes before adding non-narrowing signatures; verify the module build and smoke.

## library-typed list composition slice
Cover adjacent pairs, splitting, interleaving and deduplication before typing their generic inputs and list results; verify behavior and smoke.

# library-typed extrema helpers

Cover list indices and numeric median and minmax outcomes before adding non-narrowing signatures; verify the module and smoke on merged main.

# library-typed list statistics and construction

Cover documented list construction and statistics outcomes, then declare non-narrowing argument and result types and verify the module and smoke.

# library-typed list scans

Cover the documented scan and predicate-based take/drop outcomes, then type their generic inputs and list results and verify the module and smoke.

## library-typed callback helpers

Check callback outcomes before annotating generic parameters and results; preserve existing accepted values, verify the module and smoke on current main, and continue the library audit.

## library-typed map selection

Cover map selection and lookup outcomes before adding non-narrowing signatures; verify focused tests and merged-main smoke.

## library-typed map transformations

Cover map construction, mapping and fallback outcomes before adding generic signatures; verify focused results and merged-main smoke.

## library-typed map entries

Cover entry lists and mutating helpers before typing their generic signatures; verify direct outcomes and merged-main smoke.

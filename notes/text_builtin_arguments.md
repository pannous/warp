# Text builtin arguments

Text-reading builtins share the `text_argument` runtime helper. It has three responsibilities, in this order:

1. Propagate an Error argument with its original message.
2. Convert a Codepoint to its one-character Text representation.
3. Require the resulting node to be Text before any emitter reads its byte array.

Keeping the kind check in this shared boundary gives `byte_at`, `byte_slice`, trimming, and text-prefix/suffix operations the same `not a text` diagnostic. Optimized callers may skip the helper only when the operand is already proven to be a Text literal.

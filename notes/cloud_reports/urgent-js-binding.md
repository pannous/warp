done
# js-binding: `$document.location` superfluous value evaluation

`$who` reads a page id's value (card page-ids). Of a name that is a value already, `$` was evaluated to nothing:
`$document` and `x = 3; $x` were the bare symbols `$document` / `$x`, `f(n) := $n + 1` was "undefined variable: $n";
`$document.location` and `$Math.floor` worked but with a hint only.

Now (src/lowering/foreign/page_names.rs, `warn_superfluous`, `is_value_already`): `$name` of a variable, parameter or
function of the program, or of a JavaScript or browser global (Math, document, window, WebIDL globals), that no page id
or enclosing data node shadows, is `name`, with the warning
"$document is superfluous: document is a value already, `$` reads the value of a page id", offering the plain name.
Unknown `$foo`, `$0`, `$a` of an enclosing data node and page ids are unchanged; `$who.value`, `$xyz.innerHTML` keep
their hints.

Tests: new ffi::test_browser_dollar_names::a_superfluous_dollar_keeps_the_value_with_a_warning (3/3 in the file);
ffi::test_page_ids::the_old_dollar_forms_work_with_a_hint checks the `$document.title` warning instead of its old hint
(15/15). dollar 22/22, reference 33/33, pattern 67/67, closure 55/55, template 7/7.
ffi:: 134/145, web:: 382/389, match 136/137, route 44/48, shader 51/52: every failure (test_cargo_crates,
test_crate_errors, test_foreign_lines, test_deploy, test_uniscript, test_webgpu) fails the same on origin/main in this
container (crates, deploy, GPU), so it isn't caused by this change.
`cargo clippy --lib --bins --tests -- -D warnings` clean.

Not seen in a browser: natively `document.location` itself fails ("Cannot find package 'document'"); in the page
`$document.location` now lowers to exactly what `document.location` does.

Try: `warp eval 'x = 3; $x'` → warning, 3; `warp lower '$document.location'` = `warp lower 'document.location'`.

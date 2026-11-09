done
# replacing-float: `number sine(number x)` breaks the sine example

Replacing `float` with `number` in samples/numbers/sin.warp and sine.warp hit two bugs:

1. sin.warp: `x = x % turn` with `turn = 6.283185307179586 as number` (a float: too many digits for an exact
   decimal) failed with "turn is a float where an exact Int is expected". A `number` parameter is held exact (Kind::Int)
   and widen_to_float skipped every declared variable.
   Fix: widen_to_float (src/analyzer/variables.rs) also widens a `number` parameter, and user_function
   (src/analyzer/user_functions.rs) then declares that parameter `float`.
2. sine.warp: `sine(π / 2)` turned π/2 into the exact ratio of its 17 digits, and the polynomial gave a 200-digit
   fraction. A `number` parameter refused any float argument outright (`f(1.5f)`: "a float where an exact Int is
   expected").
   Fix: type_parameters_by_arguments declares a `number` parameter `float` when a call passes a float (as fit() in
   overloads.rs widens an Int to a Float). Exact arguments alone keep it exact: `f(1/3)` is still a rational.

Not fixed: `f(x:number) := x; f(sqrt(2))` is still refused, as ArgumentJudge only knows literal and variable kinds
(the untyped `f(x)` takes it).

Tests: new welcoming::test_c_style::test_number_parameter_holds_floats; test_c_style:: 4/4.
Filters: float 194/194, param 296/296 (10 ignored), overload 34/34, test_samples:: 27/27, test_math:: 26/26,
types:: 834/834, functions:: 821/822, numbers:: 517/518. The two failures are of this container, not of the change:
test_bend_laws (needs lean: "lean failed") and test_deep_recursion ("call stack exhausted", 8 MB stack here).
`cargo clippy --lib --bins --tests -- -D warnings` clean.

Try: `sed s/float/number/g samples/numbers/sine.warp > /tmp/s.warp; warp /tmp/s.warp` → 1.00000000066278
(sin.warp so → 841471).

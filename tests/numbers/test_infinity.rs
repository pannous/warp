// ∞ (also `\:infinity`) is the float infinity (P56 (4) default, until the user decides; ω is the hyperreal one)
use crate::is;

#[test]
fn infinity_is_a_float_value() {
	is!("∞ > 1e300", 1);
	is!("-∞ < -1e300", 1);
	is!("1.0/0.0 == ∞", 1);
	is!("\\:infinity == ∞", 1);
	is!("x = ∞; x > 7", 1);
	is!("min(3.5, ∞)", 3.5);
}

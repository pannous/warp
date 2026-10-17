// card exact-tempo-growth: the tempo is a float, so `tempo *= 1.05` a round keeps a short number instead of an exact
// ratio growing digits every round (40 rounds printed 70 digits and slowed the beat arithmetic)
#![cfg(feature = "native")]
use crate::is;

#[test]
fn a_growing_tempo_stays_a_float() {
	is!("use sound\nfor i in 1 to 40 { tempo *= 1.05 }\ntype(tempo)", "float");
	is!("use sound\ntempo = 100\ntempo *= 1.05\ntempo", 105.0);
	is!("use sound\ntempo = 60\ntempo *= 1.5\nplay(C4, 5ms)\n3 beats", 2.0);
}

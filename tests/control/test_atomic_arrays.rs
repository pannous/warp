// P44 (user, 2026-10-05: "make shared and atomic synonyms"): `atomic xs = int[n]` is `shared xs = int[n]`
use warp::is;

#[test]
fn atomic_is_shared() {
	is!("atomic xs = int[4]; xs#1 += 2; xs#1 += 3; xs#1", 5);
	is!("atomic hits = int[1]; bump(h, n) := { for i in 1 to n { h#1 += 1 }; n }; a = go bump(hits, 1000); b = go bump(hits, 1000); await a + await b; hits#1", 2000);
}

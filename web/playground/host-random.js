// random numbers in the page (a part of host.js, which says how parts work): the host words random, random_below and
// random_seed (crates/warp-runtime host_words.rs): after a seed, xorshift64* as natively, so a seeded program gives the
// same numbers in both; unseeded, Math.random
const U64 = (1n << 64n) - 1n;
let seededRandom = null;
const seedRandom = seed => { seededRandom = (BigInt.asUintN(64, seed) * 0x9E3779B97F4A7C15n & U64) | 1n; };
function nextSeeded() {
	let x = seededRandom;
	x ^= x >> 12n;
	x = (x ^ (x << 25n)) & U64;
	x ^= x >> 27n;
	seededRandom = x;
	return x * 0x2545F4914F6CDD1Dn & U64;
}
const randomFloat = () => seededRandom === null ? Math.random() : Number(nextSeeded() >> 11n) / 2 ** 53;
const randomBelow = bound => bound <= 0n ? 0n : seededRandom === null ? BigInt(Math.floor(Math.random() * Number(bound))) : nextSeeded() % bound;

addHostPart({
	started: () => { seededRandom = null; }, // each run starts unseeded
	words: () => ({
		random: randomFloat,
		random_below: randomBelow,
		random_seed: seedRandom,
	}),
});

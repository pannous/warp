// random numbers in the page (a part of host.js, which says how parts work): the host words random, random_below and
// random_seed (core/native/driver.warp): after a seed, libc's srand48/drand48 as natively, so a seeded program gives the
// same numbers in both; unseeded, Math.random
// drand48: the 48-bit linear congruential generator x' = a·x + c mod 2^48, srand48(s) starting it at s's low 32 bits
// followed by 0x330E
const U48 = (1n << 48n) - 1n;
const MULTIPLIER = 0x5DEECE66Dn;
const INCREMENT = 0xBn;
let seededRandom = null;
const seedRandom = seed => { seededRandom = (BigInt.asUintN(32, BigInt(seed)) << 16n) | 0x330En; };
function drand48() {
	seededRandom = (MULTIPLIER * seededRandom + INCREMENT) & U48;
	return Number(seededRandom) / 2 ** 48;
}
const randomFloat = () => seededRandom === null ? Math.random() : drand48();
const randomBelow = bound => bound <= 0n ? 0n : BigInt(Math.floor(randomFloat() * Number(bound)));

addHostPart({
	started: () => { seededRandom = null; }, // each run starts unseeded
	words: () => ({
		random: randomFloat,
		random_below: randomBelow,
		random_seed: seedRandom,
	}),
});

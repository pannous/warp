// Exact numbers beyond the fixnums as they cross between a module and JavaScript: a task's value, a foreign call's
// big integer, a run block's result (src/host/tasks.rs EXACT_BUILDERS, integer_handle). src/web/site.rs prepends this to the
// parts that carry them (host-tasks.js, host-foreign.js, host-compiler.js), so a site with two of them declares these
// twice: function declarations only, which a top-level const could not be (as imports.js)

// an integer as reader.js reads it (src/web/mod.rs int_payload): a fixnum, or a big one's 32-bit limbs, least significant first
function integerOfPayload(payload) {
	if (payload.int !== undefined) return BigInt(payload.int);
	const magnitude = payload.big.limbs.reduceRight((value, limb) => (value << 32n) | BigInt(limb), 0n);
	return payload.big.negative ? -magnitude : magnitude;
}

// numerator and denominator of an exact number beyond the fixnums: digits (src/web/mod.rs tree_of, Python), a big integer
// or a ratio as reader.js reads them (a task's value); undefined for a fixnum
function exactOfPayload(payload) {
	if (payload.exact) return payload.exact.map(BigInt);
	if (payload.ratio) return payload.ratio.map(integerOfPayload);
	if (payload.big) return [integerOfPayload(payload), 1n];
}

function isExactHandle(value) {
	const [fixnumMin, fixnumMax] = [-(2n ** 62n - 1n), 2n ** 62n];
	return typeof value === "bigint" && (value < fixnumMin || value > fixnumMax);
}

// the Int handle of an exact number beyond the fixnums, composed from fixnum pieces with the module's exported Int
// operations
function exactHandle(module, numerator, denominator) {
	const integer = n => {
		if (!isExactHandle(n)) return n;
		const limbs = [];
		for (let rest = n < 0n ? -n : n; rest > 0n; rest >>= 32n) limbs.push(rest & 0xffffffffn);
		const magnitude = limbs.reduceRight((handle, limb) => module.exact_add(module.int_shift_left(handle, 32n), limb), 0n);
		return n < 0n ? module.exact_sub(0n, magnitude) : magnitude;
	};
	return denominator === 1n ? integer(numerator) : module.exact_div(integer(numerator), integer(denominator));
}

// A member of a JavaScript value as `use js` reaches it, shared by the Worker's js runtime (host-foreign.js) and the page
// answering the Worker's calls of page-only globals (task-workers.js answerPageCall). Only a function declaration here:
// a site's scripts concatenate this file more than once.

// member "" constructs (`new URL(…)`, src/lowering/foreign/foreign_modules.rs CONSTRUCTOR_MEMBER), "set innerHTML" sets
// the member to the one argument (SETTER_PREFIX), "a.b" walks; arguments null read it
function jsMember(value, label, memberName, argumentValues) {
	if (memberName === "") return new value(...argumentValues);
	const set = memberName.match(/^set (.+)$/);
	if (set) return value[set[1]] = argumentValues[0];
	let owner = null;
	for (const part of memberName.split(".")) {
		if (value?.[part] === undefined) throw new Error(`js ${label}.${memberName}: ReferenceError: ${label} has no ${memberName}`);
		[owner, value] = [value, value[part]];
	}
	return argumentValues === null ? value : value.apply(owner, argumentValues);
}

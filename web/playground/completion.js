// Completion and replacement in the editor, as the Sublime Text packages Warp and Uniscript do (card g_oQgw):
// - a word being typed lists the keywords, the standard modules after `use`, and the words of the code, the examples
//   and the samples that start so, the most used first;
// - `\:` and `<:` list the uniscript entities (src/uniscript_entities.tsv, entities.tsv here) with their character, and
//   a chosen one becomes its character: `\:alpha` → α;
// - a typed `<:name>` becomes its character at its `>`; undo brings the tag back.
// Tab takes the first, Up/Down choose another, which Enter takes too (else Enter is a new line), Esc closes.

const ENTITIES_FILE = "entities.tsv"; // copied by build.sh, fetched on the first `\:` or `<:`
const ENTITY_OPENERS = /(\\:|<:)([A-Za-z][A-Za-z0-9-]*)?$/;
const TYPED_TAG = /<:([A-Za-z][A-Za-z0-9-]*)>$/;
const WORD_BEFORE = /[A-Za-z_][A-Za-z0-9_]*$/;
const WORDS = /[A-Za-z_][A-Za-z0-9_]*/g;
const USE_BEFORE = /\b(use|import|include)\s+[A-Za-z_]*$/;
const SHORTEST_PREFIX = 2; // words: a list from the second letter on
const MOST_SHOWN = 12;

let entities; // [name, characters] in the table's order, once fetched
let corpusCounts; // word → uses in the examples and samples

function loadEntities() {
	entities ??= fetch(ENTITIES_FILE).then(response => response.ok ? response.text() : "").catch(() => "")
		.then(table => table.split("\n").map(line => line.split("\t")).filter(pair => pair.length === 2));
	return entities;
}

function countWords(texts, counts = new Map()) {
	for (const text of texts) for (const [word] of text.matchAll(WORDS)) counts.set(word, (counts.get(word) ?? 0) + 1);
	return counts;
}

function corpus() {
	corpusCounts ??= countWords([...Object.values(window.EXAMPLES ?? {}).map(example => example.code ?? ""), ...Object.values(window.SAMPLES ?? {})]);
	return corpusCounts;
}

// the words starting with `prefix` (not it alone): keywords first, then by their uses in the code and the corpus
function wordChoices(editor, prefix, line) {
	if (USE_BEFORE.test(line)) return (KEYWORDS.modules ?? []).filter(name => name.startsWith(prefix)).map(name => ({ text: name }));
	const counts = countWords([editor.getValue()], new Map(corpus()));
	const keywords = new Set([...KEYWORDS.hard, ...KEYWORDS.soft]);
	keywords.forEach(word => counts.set(word, (counts.get(word) ?? 0) + 1));
	counts.set(prefix, (counts.get(prefix) ?? 1) - 1); // the word being typed counts once in the code
	return [...counts].filter(([word, uses]) => word.startsWith(prefix) && word !== prefix && uses > 0)
		.sort(([one, oneUses], [other, otherUses]) => keywords.has(other) - keywords.has(one) || otherUses - oneUses || one.localeCompare(other))
		.slice(0, MOST_SHOWN).map(([word]) => ({ text: word }));
}

async function entityChoices(prefix) {
	const table = await loadEntities();
	const starting = table.filter(([name]) => name.startsWith(prefix));
	const exact = starting.filter(([name]) => name === prefix);
	return [...exact, ...starting.filter(([name]) => name !== prefix)].slice(0, MOST_SHOWN)
		.map(([name, characters]) => ({ text: characters, label: name, shown: characters }));
}

function startCompletion(editor) {
	const list = document.createElement("ul");
	list.className = "completions";
	list.hidden = true;
	document.body.append(list);
	let open; // { from, to, choices, chosen }
	let asked = 0; // the newest request: an older one's entities arrive too late to show

	const close = () => {
		open = undefined;
		list.hidden = true;
		editor.removeKeyMap(keys);
	};
	const take = index => {
		const { from, to, choices } = open;
		close();
		editor.replaceRange(choices[index].text, from, to, "+complete");
		editor.focus();
	};
	const choose = step => {
		open.chosen = (open.chosen + step + open.choices.length) % open.choices.length;
		open.moved = true;
		showChosen();
	};
	const showChosen = () => [...list.children].forEach((item, index) => item.classList.toggle("chosen", index === open.chosen));
	const keys = {
		Up: () => choose(-1),
		Down: () => choose(1),
		Enter: () => {
			if (!open.moved) return close() ?? CodeMirror.Pass;
			take(open.chosen);
		},
		Tab: () => take(open.chosen),
		Esc: close,
	};
	const show = (from, to, choices) => {
		if (choices.length === 0) return close();
		const wasOpen = Boolean(open);
		open = { from, to, choices, chosen: 0 };
		list.replaceChildren(...choices.map((choice, index) => {
			const item = document.createElement("li");
			item.append(choice.label ?? choice.text);
			if (choice.shown) item.append(Object.assign(document.createElement("span"), { className: "glyph", textContent: choice.shown }));
			item.onmousedown = event => { event.preventDefault(); take(index); };
			return item;
		}));
		showChosen();
		const at = editor.cursorCoords(from, "page");
		Object.assign(list.style, { left: `${at.left}px`, top: `${at.bottom}px` });
		list.hidden = false;
		if (!wasOpen) editor.addKeyMap(keys);
	};

	const update = async () => {
		const cursor = editor.getCursor();
		const line = editor.getLine(cursor.line).slice(0, cursor.ch);
		const request = ++asked;
		const entity = line.match(ENTITY_OPENERS);
		if (entity) {
			const choices = await entityChoices(entity[2] ?? "");
			if (request === asked) show({ line: cursor.line, ch: cursor.ch - entity[0].length }, cursor, choices);
			return;
		}
		const word = line.match(WORD_BEFORE)?.[0] ?? "";
		const afterUse = USE_BEFORE.test(line);
		if (word.length < SHORTEST_PREFIX && !afterUse) return close();
		show({ line: cursor.line, ch: cursor.ch - word.length }, cursor, wordChoices(editor, word, line));
	};

	// `<:alpha>` typed: its character, when the name is an entity
	const replaceTypedTag = async () => {
		const cursor = editor.getCursor();
		const tag = editor.getLine(cursor.line).slice(0, cursor.ch).match(TYPED_TAG);
		if (!tag) return false;
		const found = (await loadEntities()).find(([name]) => name === tag[1]);
		if (found) editor.replaceRange(found[1], { line: cursor.line, ch: cursor.ch - tag[0].length }, cursor, "+complete");
		return Boolean(found);
	};

	editor.on("inputRead", async (_, change) => {
		if (change.text.join("").endsWith(">") && await replaceTypedTag()) return close();
		update();
	});
	editor.on("keyHandled", (_, name) => name === "Backspace" && open && update());
	editor.on("blur", close);
	editor.on("cursorActivity", () => {
		if (open && editor.getCursor().line !== open.from.line) close();
	});
	return { update, close };
}

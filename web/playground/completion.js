// Completion and replacement in the editor, as the Sublime Text packages Warp and Uniscript do (card g_oQgw):
// - a word being typed lists the words of the code in the editor (all of it: `use math` … `dir(ma` offers math), then
//   the keywords, then the words of the examples and the samples that start so, the most used first; after `use` the
//   standard modules;
// - `\:` and `<:` list the uniscript entities (src/uniscript_entities.tsv, entities.tsv here) with their character, and
//   a chosen one becomes its character: `\:alpha` → α;
// - a typed `<:name>` becomes its character at its `>`; undo brings the tag back.
// Tab and Enter take the chosen one (the first, or another chosen with Up/Down), Esc closes; Enter is a new line when
// nothing was chosen and the word typed is a whole word already (`print`, a name of the code). Tab after a word with
// no list open lists its completions from the first letter on, or takes the only one (cards code-completion-must,
// completion-must, code-completion-should).

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
	// EXAMPLES and SAMPLES are top-level consts of their scripts (index.html defines SAMPLES when samples.js is missing),
	// which are no properties of window
	corpusCounts ??= countWords([...Object.values(EXAMPLES).map(example => example.code ?? ""), ...Object.values(SAMPLES)]);
	return corpusCounts;
}

const keywordSet = () => new Set([...KEYWORDS.hard, ...KEYWORDS.soft]);

// the words of the code in the editor by their uses, the word being typed (`prefix`) counted once less
function codeCounts(editor, prefix) {
	const counts = countWords([editor.getValue()]);
	counts.set(prefix, (counts.get(prefix) ?? 1) - 1);
	return counts;
}

// the words starting with `prefix` (not it alone): the code's first, then the keywords, then the corpus's, each by uses
function wordChoices(editor, prefix, line) {
	if (USE_BEFORE.test(line)) return (KEYWORDS.modules ?? []).filter(name => name.startsWith(prefix)).map(name => ({ text: name }));
	const inCode = codeCounts(editor, prefix);
	const keywords = keywordSet();
	const counts = new Map(corpus());
	inCode.forEach((uses, word) => counts.set(word, (counts.get(word) ?? 0) + uses));
	keywords.forEach(word => counts.set(word, (counts.get(word) ?? 0) + 1));
	const rank = word => inCode.get(word) > 0 ? 2 : keywords.has(word) ? 1 : 0;
	return [...counts].filter(([word, uses]) => word.startsWith(prefix) && word !== prefix && uses > 0)
		.sort(([one, oneUses], [other, otherUses]) => rank(other) - rank(one) || otherUses - oneUses || one.localeCompare(other))
		.slice(0, MOST_SHOWN).map(([word]) => ({ text: word }));
}

// the word typed is one already, a keyword or a name of the code: Enter after it is a new line
const isWholeWord = (editor, word) => keywordSet().has(word) || codeCounts(editor, word).get(word) > 0;

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
			if (!open.moved && open.typedWhole) return close() ?? CodeMirror.Pass;
			take(open.chosen);
		},
		Tab: () => take(open.chosen),
		Esc: close,
	};
	const show = (from, to, choices, typedWhole = false) => {
		if (choices.length === 0) return close();
		const wasOpen = Boolean(open);
		open = { from, to, choices, chosen: 0, typedWhole };
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

	// the word before the cursor, the line up to it and where the word starts
	const typed = () => {
		const cursor = editor.getCursor();
		const line = editor.getLine(cursor.line).slice(0, cursor.ch);
		const word = line.match(WORD_BEFORE)?.[0] ?? "";
		return { cursor, line, word, from: { line: cursor.line, ch: cursor.ch - word.length } };
	};

	const update = async () => {
		const { cursor, line, word, from } = typed();
		const request = ++asked;
		const entity = line.match(ENTITY_OPENERS);
		if (entity) {
			const choices = await entityChoices(entity[2] ?? "");
			if (request === asked) show({ line: cursor.line, ch: cursor.ch - entity[0].length }, cursor, choices);
			return;
		}
		const afterUse = USE_BEFORE.test(line);
		if (word.length < SHORTEST_PREFIX && !afterUse) return close();
		show(from, cursor, wordChoices(editor, word, line), isWholeWord(editor, word));
	};

	// Tab after a word, no list open: its only completion taken, or its completions listed; else Tab indents
	const completeWord = () => {
		const { cursor, line, word, from } = typed();
		const choices = word ? wordChoices(editor, word, line) : [];
		if (choices.length === 0) return CodeMirror.Pass;
		if (choices.length === 1) return editor.replaceRange(choices[0].text, from, cursor, "+complete");
		show(from, cursor, choices, isWholeWord(editor, word));
	};
	editor.addKeyMap({ Tab: completeWord });

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

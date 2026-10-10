// The assistant (card g_X_Uw): an Anthropic API key pasted into the ⋯ menu, kept in this browser's localStorage only
// and sent nowhere but to api.anthropic.com, gives
// - programs `agent "…"` (lib/agent.warp): env("ANTHROPIC_API_KEY") is a stand-in the worker swaps for the key only
//   in a request to the API (host-files.js withPageSecret), so no program can send the key elsewhere
// - completion in the editor: with a key, a pause in typing at the end of a line shows Claude's continuation there in
//   gray (card g_oQnE), also while the word list (completion.js) is open, which it then closes; Ctrl-Space (or
//   Alt-Space) asks for it anywhere, Tab takes it; an answer without text says why (card code-completion)
// - a small chat about the program in the editor (Ask ✦)

const API_KEY_STORAGE = "warp-playground-anthropic-key";
const API_KEY_VARIABLE = "ANTHROPIC_API_KEY";
const API_KEY_STAND_IN = "playground-key"; // what env gives a program instead of the key
const API_ORIGIN = "https://api.anthropic.com";
const MESSAGES_URL = `${API_ORIGIN}/v1/messages`;
const API_VERSION = "2023-06-01";
const CHAT_MODEL = "claude-sonnet-5-5";
const COMPLETION_MODEL = "claude-haiku-5-5";
const CHAT_TOKENS = 2048;
const COMPLETION_TOKENS = 256;
// a continuation needs no thinking first: thinking took the 256 tokens, and the answer came without text (card code-completion)
const NO_THINKING = { type: "disabled" };
const NOTHING_SUGGESTED = "Claude suggests nothing here.";
const CURSOR_MARK = "‸";
const LANGUAGE_GUIDE = "primer.md"; // the language in short, what Claude is told about warp; the site serves it as /llms.txt too
const CODE_FENCE = /```([a-z]*)\n?([\s\S]*?)```/g; // a split gives text, language, code, text, …
const FAILURE_SHOWN_MS = 5000; // how long a failed completion's reason stays under its line
const NO_KEY = "Paste an Anthropic API key into the ⋯ menu first.";
const PAUSE_BEFORE_COMPLETION_MS = 1000; // typing paused this long at a line's end asks for a completion by itself

const apiKey = () => stored(API_KEY_STORAGE) ?? "";
const saveApiKey = key => store(API_KEY_STORAGE, key);

// what the worker gets: the stand-in as the program's environment, the key itself only to put into a request to the API
function programEnvironment() {
	const key = apiKey().trim();
	if (!key) return { environment: {}, secrets: {} };
	return { environment: { [API_KEY_VARIABLE]: API_KEY_STAND_IN }, secrets: { [API_KEY_STAND_IN]: { value: key, origin: API_ORIGIN } } };
}

let languageGuide;
const warpGuide = () => languageGuide ??= fetch(LANGUAGE_GUIDE).then(answer => answer.ok ? answer.text() : "", () => "");

// Claude's answer to the messages, as text; a failed request is the API's own reason
async function askClaude({ model, system, messages, maxTokens, thinking }) {
	const key = apiKey().trim();
	if (!key) throw new Error(NO_KEY);
	const answer = await fetch(MESSAGES_URL, {
		method: "POST",
		headers: { "x-api-key": key, "anthropic-version": API_VERSION, "content-type": "application/json", "anthropic-dangerous-direct-browser-access": "true" },
		body: JSON.stringify({ model, system, messages, max_tokens: maxTokens, thinking }),
	});
	const reply = await answer.json().catch(() => ({}));
	if (!answer.ok) throw new Error(reply.error?.message ?? `HTTP status ${answer.status}`);
	const text = reply.content?.filter(part => part.type === "text").map(part => part.text).join("") ?? "";
	if (!text && reply.stop_reason === "max_tokens") throw new Error(`Claude's answer ran out of its ${maxTokens} tokens before any text.`);
	return text;
}

async function systemPrompt(task) {
	return `You help with warp, a data format and wasm-first programming language. Its guide in short:\n\n${await warpGuide()}\n\n${task}`;
}

// ---- completion: Claude's continuation at the cursor, shown in gray until Tab takes it ----------------------------

let suggestion; // { bookmark, text, at }
let edits = 0; // changes so far: an answer to older code is not shown
let pauseTimer;
let failureShown; // an automatic completion's failure is shown once, not at every pause
let closeWordList = () => {}; // completion.js's list, closed when Claude's continuation shows, so Tab takes that

function dismissSuggestion() {
	suggestion?.bookmark.clear();
	suggestion = undefined;
}

// asked by Ctrl-Space, or by itself after a pause (`automatic`: no "asking" line, a failure said once)
async function suggestCompletion(editor, automatic = false) {
	dismissSuggestion();
	const at = editor.getCursor();
	const asked = edits;
	const code = editor.getRange({ line: 0, ch: 0 }, at) + CURSOR_MARK + editor.getRange(at, { line: editor.lastLine() });
	const shown = automatic ? undefined : editor.addLineWidget(at.line, element("div", { className: "completion-pending" }, "asking Claude…"));
	try {
		const system = await systemPrompt(`Complete the warp program at ${CURSOR_MARK}. Answer with only the text to insert there: no explanation, no code fence.`);
		const text = (await askClaude({ model: COMPLETION_MODEL, system, messages: [{ role: "user", content: code }], maxTokens: COMPLETION_TOKENS, thinking: NO_THINKING })).replace(CODE_FENCE, "$2");
		if (asked !== edits || !sameSpot(editor.getCursor(), at)) return;
		if (!text) {
			if (!automatic) throw new Error(NOTHING_SUGGESTED);
			return;
		}
		closeWordList();
		const bookmark = editor.setBookmark(at, { widget: element("span", { className: "completion" }, text), insertLeft: true });
		suggestion = { bookmark, text, at };
	} catch (failure) {
		if (automatic && failureShown === failure.message) return;
		failureShown = failure.message;
		const failed = editor.addLineWidget(at.line, element("div", { className: "completion-failed" }, failure.message));
		setTimeout(() => failed.clear(), FAILURE_SHOWN_MS);
	} finally {
		shown?.clear();
	}
}

// typing paused at the end of a line that has something on it, with a key and nothing selected; an open word list
// does not hold it back (it was open at most pauses after a word, so no continuation came)
function completeAfterPause(editor) {
	clearTimeout(pauseTimer);
	if (!apiKey().trim()) return;
	pauseTimer = setTimeout(() => {
		const at = editor.getCursor();
		const line = editor.getLine(at.line);
		if (editor.hasFocus() && !editor.somethingSelected() && at.ch === line.length && line.trim()) suggestCompletion(editor, true);
	}, PAUSE_BEFORE_COMPLETION_MS);
}

function takeSuggestion(editor) {
	if (!suggestion) return CodeMirror.Pass;
	const { text, at } = suggestion;
	dismissSuggestion();
	editor.replaceRange(text, at);
}

// ---- chat about the program in the editor ---------------------------------------------------------------------

const conversation = []; // {role, content} of this page's chat

// the program and what its last run showed, the chat's context
function currentProgram(editor) {
	const shown = ["value", "printed", "diagnostics"].map(id => $(id).textContent.trim()).filter(Boolean).join("\n");
	return `The program in the editor:\n\`\`\`warp\n${editor.getValue()}\n\`\`\`\nIts last run showed:\n${shown || "(nothing yet)"}`;
}

// ---- an answer's code into the editor as edits, so undo, the cursor and the untouched lines stay (card put-editor) ----
// An edit replaces the lines from..to (to excluded) of the editor's text by `lines`.

// the one edit that turns the current text into the wanted one: the lines between their common start and end
function replacementEdits(current, wanted) {
	// the editor's text keeps its own last line break
	const [old, lines] = [current, wanted].map(text => text.replace(/\n$/, "").split("\n"));
	let start = 0;
	while (start < old.length && start < lines.length && old[start] === lines[start]) start++;
	let end = 0;
	while (end < old.length - start && end < lines.length - start && old[old.length - 1 - end] === lines[lines.length - 1 - end]) end++;
	if (start === old.length && start === lines.length) return [];
	return [{ from: start, to: old.length - end, lines: lines.slice(start, lines.length - end) }];
}

// a unified diff's hunks as edits of the current text: each hunk's context and removed lines are found in the text,
// nearest to the line its @@ header names; a hunk whose lines are not there fails, naming its first line
function diffEdits(current, diff) {
	const old = current.split("\n");
	const hunks = [];
	for (const line of diff.replace(/\n$/, "").split("\n")) {
		const header = line.match(/^@@ -(\d+)/);
		if (header) hunks.push({ near: Number(header[1]) - 1, before: [], after: [] });
		else if (/^(---|\+\+\+|\\) /.test(line) || !hunks.length) continue;
		else {
			const hunk = hunks.at(-1);
			const [mark, text] = line === "" ? [" ", ""] : [line[0], line.slice(1)];
			if (mark !== "+") hunk.before.push(text);
			if (mark !== "-") hunk.after.push(text);
		}
	}
	const same = (one, other) => one.trimEnd() === other.trimEnd();
	const fitsAt = (lines, at) => lines.every((line, index) => same(old[at + index] ?? "", line) && at + index < old.length);
	let searchFrom = 0;
	return hunks.map(({ near, before, after }) => {
		const places = before.length ? [...old.keys()].filter(at => at >= searchFrom && fitsAt(before, at)) : [Math.min(Math.max(near, searchFrom), old.length)];
		if (!places.length) throw new Error(`The change no longer fits the editor's program: "${before.find(line => line.trim()) ?? ""}" is not there.`);
		const at = places.reduce((best, place) => Math.abs(place - near) < Math.abs(best - near) ? place : best);
		searchFrom = at + before.length;
		return { from: at, to: at + before.length, lines: after };
	});
}

const DIFF_LANGUAGES = ["diff", "patch"];
const answerEdits = (current, language, code) => DIFF_LANGUAGES.includes(language) ? diffEdits(current, code) : replacementEdits(current, code);

// an edit as a CodeMirror range of a text of `lineCount` lines and `lineLength(i)`: lines removed at the end take the
// line break before them along, lines added at the end get one
function editRange({ from, to, lines }, lineCount, lineLength) {
	const text = lines.join("\n");
	if (to < lineCount) return { start: { line: from, ch: 0 }, end: { line: to, ch: 0 }, text: lines.length ? text + "\n" : "" };
	const end = { line: lineCount - 1, ch: lineLength(lineCount - 1) };
	if (from === 0) return { start: { line: 0, ch: 0 }, end, text };
	return { start: { line: from - 1, ch: lineLength(from - 1) }, end, text: lines.length ? "\n" + text : "" };
}

// the edits made in the editor in one step (one undo), the last first so earlier line numbers still hold
function applyEdits(editor, edits) {
	editor.operation(() => [...edits].reverse().forEach(edit => {
		const { start, end, text } = editRange(edit, editor.lineCount(), line => editor.getLine(line).length);
		editor.replaceRange(text, start, end, "+assistant");
	}));
	if (edits.length) editor.scrollIntoView({ line: edits[0].from, ch: 0 }, 40);
}

// an answer's text, each code block with a button that applies it to the editor: a diff changes the lines it names,
// a whole program replaces only the lines that differ
function answerElement(editor, text) {
	const parts = text.split(CODE_FENCE);
	const shown = [];
	for (let index = 0; index < parts.length; index += 3) {
		shown.push(element("p", {}, parts[index].trim()));
		if (index + 2 >= parts.length) break;
		const [language, code] = [parts[index + 1], parts[index + 2]];
		const failure = element("div", { className: "chat-failed", hidden: true });
		const apply = () => {
			try {
				applyEdits(editor, answerEdits(editor.getValue(), language, code));
				failure.hidden = true;
			} catch (problem) {
				failure.textContent = problem.message;
				failure.hidden = false;
			}
		};
		const label = DIFF_LANGUAGES.includes(language) ? "apply to the editor" : "put into the editor";
		shown.push(element("div", { className: "chat-code" }, element("pre", {}, code), element("button", { type: "button", onclick: apply }, label), failure));
	}
	return element("div", { className: "chat-answer" }, ...shown);
}

// the chat starts anew: nothing shown, nothing sent along with the next question
function clearChat() {
	conversation.length = 0;
	$("chat-log").replaceChildren();
	$("chat-input").focus();
}

async function sendChat(editor, question) {
	const log = $("chat-log");
	log.append(element("div", { className: "chat-question" }, question));
	conversation.push({ role: "user", content: question });
	const waiting = element("div", { className: "chat-pending" }, "…");
	log.append(waiting);
	try {
		const system = await systemPrompt(`Answer questions about this program, briefly. Give warp code in fenced blocks. To change the program in the editor, give the change as a unified diff of it in a \`\`\`diff block (@@ headers with its line numbers, a few lines of context); a new program as a whole \`\`\`warp block.\n\n${currentProgram(editor)}`);
		const answer = await askClaude({ model: CHAT_MODEL, system, messages: conversation, maxTokens: CHAT_TOKENS });
		if (!waiting.isConnected) return; // the chat was cleared meanwhile
		conversation.push({ role: "assistant", content: answer });
		waiting.replaceWith(answerElement(editor, answer));
	} catch (failure) {
		if (waiting.isConnected) conversation.pop();
		waiting.replaceWith(element("div", { className: "chat-failed" }, failure.message));
	}
	log.scrollTop = log.scrollHeight;
}

// the key field, the editor's completion keys, the chat; `keyChanged` tells the worker the new environment,
// `wordList` is the editor's word completion (completion.js startCompletion)
function startAssistant(editor, keyChanged, wordList) {
	closeWordList = wordList.close;
	$("api-key").value = apiKey();
	$("api-key").onchange = event => { saveApiKey(event.target.value.trim()); failureShown = undefined; keyChanged(); };
	editor.addKeyMap({ "Ctrl-Space": suggestCompletion, "Alt-Space": suggestCompletion, Tab: takeSuggestion, Esc: dismissSuggestion });
	editor.on("change", (_, change) => {
		edits++;
		dismissSuggestion();
		if (change.origin === "+input" || change.origin === "+delete") completeAfterPause(editor);
	});
	editor.on("cursorActivity", () => suggestion && !sameSpot(editor.getCursor(), suggestion.at) && dismissSuggestion());
	$("ask").onclick = () => { $("assistant").hidden = !$("assistant").hidden; $("chat-input").focus(); };
	$("chat-clear").onclick = clearChat;
	$("chat-form").onsubmit = event => {
		event.preventDefault();
		const question = $("chat-input").value.trim();
		if (!question) return;
		$("chat-input").value = "";
		sendChat(editor, question);
	};
}

const sameSpot = (one, other) => one.line === other.line && one.ch === other.ch;

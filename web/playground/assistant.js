// The assistant (card g_X_Uw): an Anthropic API key pasted into the ⋯ menu, kept in this browser's localStorage only
// and sent nowhere but to api.anthropic.com, gives
// - programs `agent "…"` (lib/agent.warp): env("ANTHROPIC_API_KEY") is a stand-in the worker swaps for the key only
//   in a request to the API (host-files.js withPageSecret), so no program can send the key elsewhere
// - completion in the editor: Ctrl-Space (or Alt-Space) shows Claude's continuation at the cursor, Tab takes it
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
const CURSOR_MARK = "‸";
const LANGUAGE_GUIDE = "guide-expert.md"; // the language in short, what Claude is told about warp
const CODE_FENCE = /```[a-z]*\n?([\s\S]*?)```/g;
const FAILURE_SHOWN_MS = 5000; // how long a failed completion's reason stays under its line
const NO_KEY = "Paste an Anthropic API key into the ⋯ menu first.";

const apiKey = () => { try { return localStorage.getItem(API_KEY_STORAGE) ?? ""; } catch { return ""; } };

function saveApiKey(key) {
	try { key ? localStorage.setItem(API_KEY_STORAGE, key) : localStorage.removeItem(API_KEY_STORAGE); } catch { /* private window: lasts for this page */ }
}

// what the worker gets: the stand-in as the program's environment, the key itself only to put into a request to the API
function programEnvironment() {
	const key = apiKey().trim();
	if (!key) return { environment: {}, secrets: {} };
	return { environment: { [API_KEY_VARIABLE]: API_KEY_STAND_IN }, secrets: { [API_KEY_STAND_IN]: { value: key, origin: API_ORIGIN } } };
}

let languageGuide;
const warpGuide = () => languageGuide ??= fetch(LANGUAGE_GUIDE).then(answer => answer.ok ? answer.text() : "", () => "");

// Claude's answer to the messages, as text; a failed request is the API's own reason
async function askClaude({ model, system, messages, maxTokens }) {
	const key = apiKey().trim();
	if (!key) throw new Error(NO_KEY);
	const answer = await fetch(MESSAGES_URL, {
		method: "POST",
		headers: { "x-api-key": key, "anthropic-version": API_VERSION, "content-type": "application/json", "anthropic-dangerous-direct-browser-access": "true" },
		body: JSON.stringify({ model, system, messages, max_tokens: maxTokens }),
	});
	const reply = await answer.json().catch(() => ({}));
	if (!answer.ok) throw new Error(reply.error?.message ?? `HTTP status ${answer.status}`);
	return reply.content?.filter(part => part.type === "text").map(part => part.text).join("") ?? "";
}

async function systemPrompt(task) {
	return `You help with warp, a data format and wasm-first programming language. Its guide in short:\n\n${await warpGuide()}\n\n${task}`;
}

// ---- completion: Claude's continuation at the cursor, shown in gray until Tab takes it ----------------------------

let suggestion; // { bookmark, text, at }

function dismissSuggestion() {
	suggestion?.bookmark.clear();
	suggestion = undefined;
}

async function suggestCompletion(editor) {
	dismissSuggestion();
	const at = editor.getCursor();
	const code = editor.getRange({ line: 0, ch: 0 }, at) + CURSOR_MARK + editor.getRange(at, { line: editor.lastLine() });
	const shown = editor.addLineWidget(at.line, element("div", { className: "completion-pending" }, "asking Claude…"));
	try {
		const system = await systemPrompt(`Complete the warp program at ${CURSOR_MARK}. Answer with only the text to insert there: no explanation, no code fence.`);
		const text = (await askClaude({ model: COMPLETION_MODEL, system, messages: [{ role: "user", content: code }], maxTokens: COMPLETION_TOKENS })).replace(CODE_FENCE, "$1");
		if (!text || editor.getCursor().line !== at.line || editor.getCursor().ch !== at.ch) return;
		const bookmark = editor.setBookmark(at, { widget: element("span", { className: "completion" }, text), insertLeft: true });
		suggestion = { bookmark, text, at };
	} catch (failure) {
		const failed = editor.addLineWidget(at.line, element("div", { className: "completion-failed" }, failure.message));
		setTimeout(() => failed.clear(), FAILURE_SHOWN_MS);
	} finally {
		shown.clear();
	}
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

// an answer's text, each code block with a button that puts it into the editor
function answerElement(editor, text) {
	const parts = text.split(CODE_FENCE);
	return element("div", { className: "chat-answer" }, ...parts.map((part, index) => index % 2 === 0
		? element("p", {}, part.trim())
		: element("div", { className: "chat-code" }, element("pre", {}, part), element("button", { type: "button", onclick: () => editor.setValue(part) }, "put into the editor"))));
}

async function sendChat(editor, question) {
	const log = $("chat-log");
	log.append(element("div", { className: "chat-question" }, question));
	conversation.push({ role: "user", content: question });
	const waiting = element("div", { className: "chat-pending" }, "…");
	log.append(waiting);
	try {
		const system = await systemPrompt(`Answer questions about this program, briefly. Give warp code in fenced blocks.\n\n${currentProgram(editor)}`);
		const answer = await askClaude({ model: CHAT_MODEL, system, messages: conversation, maxTokens: CHAT_TOKENS });
		conversation.push({ role: "assistant", content: answer });
		waiting.replaceWith(answerElement(editor, answer));
	} catch (failure) {
		conversation.pop();
		waiting.replaceWith(element("div", { className: "chat-failed" }, failure.message));
	}
	log.scrollTop = log.scrollHeight;
}

// the key field, the editor's completion keys, the chat; `keyChanged` tells the worker the new environment
function startAssistant(editor, keyChanged) {
	$("api-key").value = apiKey();
	$("api-key").onchange = event => { saveApiKey(event.target.value.trim()); keyChanged(); };
	editor.addKeyMap({ "Ctrl-Space": suggestCompletion, "Alt-Space": suggestCompletion, Tab: takeSuggestion, Esc: dismissSuggestion });
	editor.on("change", () => suggestion && dismissSuggestion());
	editor.on("cursorActivity", () => suggestion && !sameSpot(editor.getCursor(), suggestion.at) && dismissSuggestion());
	$("ask").onclick = () => { $("assistant").hidden = !$("assistant").hidden; $("chat-input").focus(); };
	$("chat-form").onsubmit = event => {
		event.preventDefault();
		const question = $("chat-input").value.trim();
		if (!question) return;
		$("chat-input").value = "";
		sendChat(editor, question);
	};
}

const sameSpot = (one, other) => one.line === other.line && one.ch === other.ch;

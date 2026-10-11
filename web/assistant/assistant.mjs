// warp-assistant: the playground's assistant for visitors without an Anthropic key of their own (card assistant-proxy).
// No open relay: it answers only the playground's Origin, after a Turnstile check (POST /session gives a session token
// for an hour), rate-limited per IP; it pins the model and each task's max_tokens and builds the system prompt itself
// from web/playground/assistant.json, so a caller chooses only the task, the messages and, for chat, the program.
// POST /messages answers as the Messages API does (errors too: { error: { message } }).
// Secrets (wrangler secret put …): ANTHROPIC_API_KEY (from its own Console workspace, with a monthly spend limit),
// TURNSTILE_SECRET, SESSION_SECRET. Deploy: wrangler deploy (in this folder). Test: node web/assistant/test_assistant.mjs
import ASSISTANT from "../playground/assistant.json";

const MESSAGES_URL = "https://api.anthropic.com/v1/messages";
const API_VERSION = "2023-06-01";
const SITEVERIFY_URL = "https://challenges.cloudflare.com/turnstile/v0/siteverify";
const SESSION_SECONDS = 3600;
const GUIDE_KEPT_MS = 3600 * 1000; // the playground's guides are read again after this
const MAX_MESSAGES = 24;
const MAX_INPUT_CHARACTERS = 24000; // the messages' and the program's text together
const ROLES = ["user", "assistant"];
const NO_THINKING = { type: "disabled" };
const NOT_SET_UP = "The playground's shared assistant is not set up yet: paste your own Anthropic API key into the ⋯ menu.";

export default {
	async fetch(request, env) {
		const origin = request.headers.get("origin");
		if (origin !== env.PLAYGROUND_ORIGIN) return failure(403, "This assistant answers only the warp playground.");
		const answer = request.method === "OPTIONS" ? preflight() : await answered(request, env).catch(problem => problem instanceof Response ? problem : failure(500, problem.message));
		answer.headers.set("access-control-allow-origin", origin);
		answer.headers.set("vary", "origin");
		return answer;
	},
};

const failure = (status, message) => Response.json({ type: "error", error: { type: "proxy", message } }, { status });

const preflight = () => new Response(null, { headers: { "access-control-allow-methods": "POST", "access-control-allow-headers": "authorization, content-type", "access-control-max-age": "86400" } });

async function answered(request, env) {
	if (request.method !== "POST") throw failure(405, "POST /session or /messages.");
	if (!env.ANTHROPIC_API_KEY || !env.TURNSTILE_SECRET || !env.SESSION_SECRET) throw failure(503, NOT_SET_UP);
	const address = request.headers.get("cf-connecting-ip") ?? "";
	if (!(await env.PER_ADDRESS.limit({ key: address })).success) throw failure(429, "Too many requests from this address: wait a minute.");
	const body = await request.json().catch(() => { throw failure(400, "The request is not JSON."); });
	const path = new URL(request.url).pathname;
	if (path === "/session") return Response.json({ session: await openSession(body.turnstile, address, env) });
	if (path !== "/messages") throw failure(404, "POST /session or /messages.");
	await checkSession(request, address, env);
	return claudeAnswer(body, env);
}

// ---- sessions: one Turnstile check, then a token signed for this address that holds an hour --------------------------

async function openSession(turnstileToken, address, env) {
	const form = new FormData();
	form.append("secret", env.TURNSTILE_SECRET);
	form.append("response", String(turnstileToken ?? ""));
	form.append("remoteip", address);
	const outcome = await (await fetch(SITEVERIFY_URL, { method: "POST", body: form })).json();
	if (!outcome.success) throw failure(403, `The bot check failed (${outcome["error-codes"]?.join(", ")}).`);
	const expires = Math.floor(Date.now() / 1000) + SESSION_SECONDS;
	return `${expires}.${await signature(`${expires}.${address}`, env.SESSION_SECRET)}`;
}

async function checkSession(request, address, env) {
	const [expires, signed = ""] = (request.headers.get("authorization") ?? "").replace(/^Bearer /, "").split(".");
	const expected = await signature(`${expires}.${address}`, env.SESSION_SECRET);
	const encoder = new TextEncoder();
	const valid = Number(expires) > Date.now() / 1000 && signed.length === expected.length && crypto.subtle.timingSafeEqual(encoder.encode(signed), encoder.encode(expected));
	if (!valid) throw failure(401, "No session, or an expired one: the playground opens a new one.");
}

async function signature(text, secret) {
	const encoder = new TextEncoder();
	const key = await crypto.subtle.importKey("raw", encoder.encode(secret), { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
	const signed = new Uint8Array(await crypto.subtle.sign("HMAC", key, encoder.encode(text)));
	return btoa(String.fromCharCode(...signed)).replaceAll("+", "-").replaceAll("/", "_").replaceAll("=", "");
}

// ---- the answer: the task's prompt, model and tokens are this Worker's, only the messages are the caller's ------------

async function claudeAnswer({ task, messages, program = "" }, env) {
	if (!Object.hasOwn(ASSISTANT.tasks, task)) throw failure(400, `Unknown task ${task}: one of ${Object.keys(ASSISTANT.tasks).join(", ")}.`);
	const wellFormed = Array.isArray(messages) && messages.length > 0 && messages.length <= MAX_MESSAGES && typeof program === "string"
		&& messages.every(message => ROLES.includes(message?.role) && typeof message.content === "string");
	if (!wellFormed) throw failure(400, `messages: 1 to ${MAX_MESSAGES} of { role: user or assistant, content: text }; program: text.`);
	const characters = messages.reduce((sum, message) => sum + message.content.length, program.length);
	if (characters > MAX_INPUT_CHARACTERS) throw failure(413, `The program and the messages are longer than ${MAX_INPUT_CHARACTERS} characters.`);
	const { tokens, thinking } = ASSISTANT.tasks[task];
	const reply = await fetch(MESSAGES_URL, {
		method: "POST",
		headers: { "x-api-key": env.ANTHROPIC_API_KEY, "anthropic-version": API_VERSION, "content-type": "application/json" },
		body: JSON.stringify({
			model: ASSISTANT.proxy_model,
			system: await systemPrompt(task, program, env),
			messages: messages.map(({ role, content }) => ({ role, content })),
			max_tokens: tokens,
			thinking: thinking === false ? NO_THINKING : undefined,
		}),
	});
	return new Response(reply.body, { status: reply.status, headers: { "content-type": "application/json" } });
}

let guide; // { text, until }: the playground's guides as last read

async function warpGuide(env) {
	if (guide?.until > Date.now()) return guide.text;
	const texts = await Promise.all(ASSISTANT.guides.map(async file => {
		const answer = await fetch(`${env.PLAYGROUND_ORIGIN}/${file}`);
		if (!answer.ok) throw failure(502, `The playground's ${file} could not be read (HTTP status ${answer.status}).`);
		return answer.text();
	}));
	guide = { text: texts.filter(Boolean).join("\n\n"), until: Date.now() + GUIDE_KEPT_MS };
	return guide.text;
}

// the same as web/playground/assistant.js systemPrompt, which builds it for a pasted key
async function systemPrompt(task, program, env) {
	const { intro, tasks } = ASSISTANT;
	return [intro, await warpGuide(env), tasks[task].prompt, tasks[task].program ? program : ""].filter(Boolean).join("\n\n");
}

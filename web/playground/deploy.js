// Deploy in the ⋯ menu (notes/hosting.md): the program's routes (`get "/" {…}`) on the web through warp-hosting
// (web/hosting). "Deploy" logs in with GitHub and hosts the program at warp-<name>.pannous.workers.dev; "Deploy to my
// Cloudflare" logs in with Cloudflare, or takes a pasted API token, and puts it in the visitor's own account. The
// compiler's worker builds the Worker's module (worker.js workerBundle); sessions and tokens stay in this browser.
// "Deploy to pannous.com" sends the source itself, with the same GitHub login, to warp-lambda
// (web/hosting/server/warp_lambda.py): `warp --sandbox serve` natively on pannous.com at <name>.lambda.pannous.com.

const HOSTING = new URL(location.href).searchParams.get("hosting") ?? "https://lambda.pannous.com";
const NAME_PATTERN = /^[a-z0-9]([a-z0-9-]{0,38}[a-z0-9])?$/; // web/hosting/hosting.mjs NAME_PATTERN
const SESSION_KEYS = { github: "warp-hosting-session", cloudflare: "warp-hosting-cloudflare" };
const NAME_KEY = "warp-hosting-name";
const PASTED_TOKEN_KEY = "warp-hosting-cloudflare-pasted";
const LOGIN_WINDOW = "width=520,height=720";
const LOGIN_TIMEOUT_MS = 5 * 60 * 1000;
const TICKET_POLL_MS = 1500; // hosting.mjs TICKET_POLL_MS
const TICKET_BYTES = 24;
const DEPLOYING_TEXT = "deploying …";
const REFUSED_LOGIN = [401, 403];

// localStorage may be unavailable (a private window): then nothing is remembered
function stored(key) {
	try { return localStorage.getItem(key); } catch { return null; }
}
function store(key, value) {
	try { value ? localStorage.setItem(key, value) : localStorage.removeItem(key); } catch { /* not remembered */ }
}

const pastedToken = () => $("cloudflare-token").value.trim();
const needsLogin = provider => !(provider === "cloudflare" && pastedToken()) && !stored(SESSION_KEYS[provider]);
const newTicket = () => btoa(String.fromCharCode(...crypto.getRandomValues(new Uint8Array(TICKET_BYTES)))).replace(/\+/g, "-").replace(/\//g, "_");
const ticketAddress = ticket => `${HOSTING}/ticket?ticket=${ticket}`;

// the one window a deploy opens, while the click still lets the page open one: the login popup when a login is
// needed, which turns into the program's tab once it is deployed (hosting.mjs handOver), else a tab waiting for it
function programWindow(provider) {
	const ticket = newTicket();
	if (needsLogin(provider)) {
		const popup = open(`${HOSTING}/auth/${provider}?origin=${encodeURIComponent(location.origin)}&ticket=${ticket}`, "warp-hosting-login", LOGIN_WINDOW);
		if (!popup) throw new Error("the login window was blocked: allow pop-ups for this page");
		const told = fields => fetch(`${ticketAddress(ticket)}&${new URLSearchParams(fields)}`, { method: "POST" }).catch(() => {});
		return { ticket, show: program => told({ program }), fail: error => told({ error }) };
	}
	const tab = open("", "_blank");
	try { tab.document.body.textContent = DEPLOYING_TEXT; } catch { /* blocked or not ours to write */ }
	return { ticket, show: program => tab ? tab.location.replace(program) : open(program, "_blank"), fail: () => tab?.close() };
}

// the login's answer (hosting.mjs handOver), {session, login} from GitHub, {cloudflareToken} from Cloudflare: posted
// by the popup while it still has its opener, else fetched by the ticket (GitHub's login page cuts the opener off)
function loggedIn(ticket) {
	return new Promise((resolve, reject) => {
		const deadline = Date.now() + LOGIN_TIMEOUT_MS;
		let polling;
		const finish = settle => {
			clearTimeout(polling);
			removeEventListener("message", answered);
			settle();
		};
		const answer = message => {
			if (message?.warpHosting) finish(() => resolve(message.warpHosting));
			else if (message?.error) finish(() => reject(new Error(message.error)));
		};
		const answered = ({ origin, data }) => origin === new URL(HOSTING).origin && answer(data);
		const poll = async () => {
			const { login } = await (await fetch(ticketAddress(ticket))).json().catch(() => ({}));
			if (login) return answer(login);
			if (Date.now() > deadline) return finish(() => reject(new Error("no login within 5 minutes")));
			polling = setTimeout(poll, TICKET_POLL_MS);
		};
		addEventListener("message", answered);
		poll();
	});
}

// the headers that say who deploys: the GitHub session, else the Cloudflare token (pasted, else logged in)
async function credentials(provider, ticket) {
	const pasted = pastedToken();
	store(PASTED_TOKEN_KEY, pasted);
	if (provider === "cloudflare" && pasted) return { "x-cloudflare-token": pasted };
	let secret = stored(SESSION_KEYS[provider]);
	if (!secret) {
		const login = await loggedIn(ticket);
		secret = login.session ?? login.cloudflareToken;
		store(SESSION_KEYS[provider], secret);
	}
	return provider === "github" ? { authorization: `Bearer ${secret}` } : { "x-cloudflare-token": secret };
}

// the compiler's worker answers {type: "bundle"} (playground.js hands it to `bundled`)
let bundleWaiting;
function bundled(bundle) {
	bundleWaiting?.(bundle);
	bundleWaiting = undefined;
}
const bundleOf = code => new Promise(resolve => {
	bundleWaiting = resolve;
	worker.postMessage({ bundle: code });
});

async function upload(provider, address, body, ticket) {
	const reply = await fetch(address, { method: "POST", body, headers: await credentials(provider, ticket) });
	const answer = await reply.json().catch(() => ({ error: `HTTP ${reply.status}` }));
	// an expired session or token: the next click logs in again (only a click may open the login window)
	if (REFUSED_LOGIN.includes(reply.status) && !pastedToken()) {
		store(SESSION_KEYS[provider], null);
		throw new Error(`${answer.error ?? "the login expired"}: press the button again to log in`);
	}
	if (!reply.ok) throw new Error(answer.error ?? `HTTP ${reply.status}`);
	return answer.url;
}

// [login provider, what is sent where]: a Worker module built from the source, or the source itself (native)
const TARGETS = {
	github: ["github", "deploying to warp-hosting…", workerUpload],
	cloudflare: ["cloudflare", "deploying to your Cloudflare account…", workerUpload],
	native: ["github", "deploying to pannous.com…", async name => [`${HOSTING}/native/deploy?name=${name}`, editor.getValue()]],
};

async function workerUpload(name) {
	setStatus("building the Worker…");
	const { module, scripts, error } = await bundleOf(editor.getValue());
	if (error) throw new Error(error);
	return [`${HOSTING}/deploy?name=${name}&scripts=${scripts.join(",")}`, module];
}

async function deploy(target) {
	const name = $("deploy-name").value.trim();
	if (!NAME_PATTERN.test(name)) return setStatus("a name to deploy as: lower-case letters, digits and dashes", true);
	store(NAME_KEY, name);
	const [provider, deploying, request] = TARGETS[target];
	let opened;
	try {
		opened = programWindow(provider);
		const [address, body] = await request(name);
		setStatus(deploying);
		const url = await upload(provider, address, body, opened.ticket);
		setStatus(`deployed: ${url}`);
		$("deployed").replaceChildren(element("a", { href: url, target: "_blank", rel: "noopener" }, url));
		opened.show(url);
	} catch (failure) {
		setStatus(`not deployed: ${failure.message}`, true);
		opened?.fail(failure.message);
	}
}

$("deploy-name").value = stored(NAME_KEY) ?? "";
$("cloudflare-token").value = stored(PASTED_TOKEN_KEY) ?? "";
$("deploy").onclick = () => deploy("github");
$("deploy-own").onclick = () => deploy("cloudflare");
$("deploy-native").onclick = () => deploy("native");

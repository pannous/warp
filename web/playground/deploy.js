// Deploy in the ⋯ menu (notes/hosting.md): the program's routes (`get "/" {…}`) on the web through warp-hosting
// (web/hosting). "Deploy" logs in with GitHub and hosts the program at warp-<name>.pannous.workers.dev; "Deploy to my
// Cloudflare" logs in with Cloudflare, or takes a pasted API token, and puts it in the visitor's own account. The
// compiler's worker builds the Worker's module (worker.js workerBundle); sessions and tokens stay in this browser.

const HOSTING = new URL(location.href).searchParams.get("hosting") ?? "https://lambda.pannous.com";
const NAME_PATTERN = /^[a-z0-9]([a-z0-9-]{0,38}[a-z0-9])?$/; // web/hosting/hosting.mjs NAME_PATTERN
const SESSION_KEYS = { github: "warp-hosting-session", cloudflare: "warp-hosting-cloudflare" };
const NAME_KEY = "warp-hosting-name";
const PASTED_TOKEN_KEY = "warp-hosting-cloudflare-pasted";
const LOGIN_WINDOW = "width=520,height=720";
const LOGIN_CLOSED_CHECK_MS = 500;
const REFUSED_LOGIN = [401, 403];

// localStorage may be unavailable (a private window): then nothing is remembered
function stored(key) {
	try { return localStorage.getItem(key); } catch { return null; }
}
function store(key, value) {
	try { value ? localStorage.setItem(key, value) : localStorage.removeItem(key); } catch { /* not remembered */ }
}

// the login popup's answer (hosting.mjs handOver): {session, login} from GitHub, {cloudflareToken} from Cloudflare
function loggedIn(provider) {
	return new Promise((resolve, reject) => {
		const popup = open(`${HOSTING}/auth/${provider}?origin=${encodeURIComponent(location.origin)}`, "warp-hosting-login", LOGIN_WINDOW);
		if (!popup) return reject(new Error("the login window was blocked: allow pop-ups for this page"));
		const closed = setInterval(() => popup.closed && finish(() => reject(new Error("the login window was closed"))), LOGIN_CLOSED_CHECK_MS);
		const answered = ({ source, data }) => {
			if (source !== popup) return;
			if (data?.warpHosting) finish(() => resolve(data.warpHosting));
			else if (data?.error) finish(() => reject(new Error(data.error)));
		};
		const finish = settle => {
			clearInterval(closed);
			removeEventListener("message", answered);
			settle();
		};
		addEventListener("message", answered);
	});
}

// the headers that say who deploys: the GitHub session, else the Cloudflare token (pasted, else logged in)
async function credentials(provider, fresh) {
	const pasted = $("cloudflare-token").value.trim();
	store(PASTED_TOKEN_KEY, pasted);
	if (provider === "cloudflare" && pasted) return { "x-cloudflare-token": pasted };
	let secret = !fresh && stored(SESSION_KEYS[provider]);
	if (!secret) {
		const login = await loggedIn(provider);
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

async function upload(provider, name, { module, scripts }, fresh = false) {
	const address = `${HOSTING}/deploy?name=${name}&scripts=${scripts.join(",")}`;
	const reply = await fetch(address, { method: "POST", body: module, headers: await credentials(provider, fresh) });
	const answer = await reply.json().catch(() => ({ error: `HTTP ${reply.status}` }));
	// an expired session or token: log in once more
	if (REFUSED_LOGIN.includes(reply.status) && !fresh && !$("cloudflare-token").value.trim()) return upload(provider, name, { module, scripts }, true);
	if (!reply.ok) throw new Error(answer.error ?? `HTTP ${reply.status}`);
	return answer.url;
}

async function deploy(provider) {
	const name = $("deploy-name").value.trim();
	if (!NAME_PATTERN.test(name)) return setStatus("a name to deploy as: lower-case letters, digits and dashes", true);
	store(NAME_KEY, name);
	setStatus("building the Worker…");
	const bundle = await bundleOf(editor.getValue());
	if (bundle.error) return setStatus(bundle.error, true);
	setStatus(provider === "github" ? "deploying to warp-hosting…" : "deploying to your Cloudflare account…");
	try {
		const url = await upload(provider, name, bundle);
		setStatus(`deployed: ${url}`);
		$("deployed").replaceChildren(element("a", { href: url, target: "_blank", rel: "noopener" }, url));
	} catch (failure) {
		setStatus(`not deployed: ${failure.message}`, true);
	}
}

$("deploy-name").value = stored(NAME_KEY) ?? "";
$("cloudflare-token").value = stored(PASTED_TOKEN_KEY) ?? "";
$("deploy").onclick = () => deploy("github");
$("deploy-own").onclick = () => deploy("cloudflare");

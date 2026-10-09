#!/usr/bin/env python3
"""warp-lambda: warp programs hosted natively on pannous.com (notes/hosting.md "Ferron", card ferron-hosting).

Ferron (pannous-lockdown ferron.kdl) hands this daemon
- lambda.pannous.com/native/…: POST /native/deploy?name=x (body: the program's source) and DELETE /native/deploy?name=x,
  logged in with GitHub as for warp-hosting (the Worker's /me says who), and GET /native/ask?domain=… (Ferron's
  on-demand TLS asks whether a certificate may be issued: only for a deployed name);
- <name>.lambda.pannous.com/…: proxied to the program, `warp --sandbox serve` on 127.0.0.2 (user decision 2026-10-09:
  sandboxed: no C, shell or other runtime, files and SQLite in its own folder, a systemd unit around it,
  warp-lambda@.service).
WARP_LAMBDA_RUNNER=process runs the programs as plain child processes instead (tests: web/hosting/test_lambda.mjs).
"""
import http.client
import http.server
import json
import os
import re
import subprocess
import sys
import threading
import time
import urllib.parse
import urllib.request

PORT = int(os.environ.get("WARP_LAMBDA_PORT", "8890"))
STATE = os.environ.get("WARP_LAMBDA_STATE", "/var/lib/warp-lambda")
DOMAIN = os.environ.get("WARP_LAMBDA_DOMAIN", "lambda.pannous.com")
IDENTITY = os.environ.get("WARP_LAMBDA_IDENTITY", "https://warp-hosting.pannous.workers.dev/me")
WARP = os.environ.get("WARP_LAMBDA_WARP", "/usr/local/bin/warp")
RUNNER = os.environ.get("WARP_LAMBDA_RUNNER", "systemd")
PROGRAM_ADDRESS = os.environ.get("WARP_LAMBDA_PROGRAM_ADDRESS", "127.0.0.2")
FIRST_PROGRAM_PORT = 9100
MAX_PROGRAMS = 20  # 3.7 GB, 2 cores: each program's unit may take 192 MB
MAX_PROGRAMS_PER_USER = 5  # as warp-hosting's (web/hosting/hosting.mjs)
MAX_SOURCE_BYTES = 256 * 1024
START_TRIES = 40
START_PAUSE = 0.25
PROXY_TIMEOUT = 30
NAME_PATTERN = re.compile(r"^[a-z0-9]([a-z0-9-]{0,38}[a-z0-9])?$")  # web/hosting/hosting.mjs NAME_PATTERN
ALLOWED_ORIGINS = [re.compile(r"^https://warp\.pannous\.com$"), re.compile(r"^https://pannous\.github\.io$"), re.compile(r"^http://(localhost|127\.0\.0\.1)(:\d+)?$")]
NATIVE = "/native"
UNIT = "warp-lambda@{}.service"
SOURCE = "app.warp"
LOG = "log"
WARP_ARGUMENTS = ["--sandbox", "--no-ask", "--no-hints", "serve"]  # warp-lambda@.service ExecStart says the same
HOP_HEADERS = {"connection", "keep-alive", "transfer-encoding", "upgrade", "proxy-connection", "te", "trailer"}

NAMES = os.path.join(STATE, "names.json")
PROGRAMS = os.path.join(STATE, "programs")
lock = threading.Lock()
children = {}  # name → Popen, WARP_LAMBDA_RUNNER=process


class Refusal(Exception):
    def __init__(self, status, message):
        super().__init__(message)
        self.status = status


def names():
    try:
        with open(NAMES) as file:
            return json.load(file)
    except FileNotFoundError:
        return {}


def save(table):
    temporary = NAMES + ".new"
    with open(temporary, "w") as file:
        json.dump(table, file, indent=1)
    os.replace(temporary, NAMES)


def user(authorization):
    """{id, login} of the GitHub login behind the header, as warp-hosting's /me says"""
    if not authorization:
        raise Refusal(401, "log in with GitHub first")
    request = urllib.request.Request(IDENTITY, headers={"authorization": authorization, "user-agent": "warp-lambda"})
    try:
        with urllib.request.urlopen(request, timeout=10) as reply:
            return json.load(reply)
    except urllib.error.HTTPError as refused:
        raise Refusal(401, json.loads(refused.read() or b"{}").get("error", "log in with GitHub again"))


def program_folder(name):
    return os.path.join(PROGRAMS, name)


def free_port(table):
    taken = {entry["port"] for entry in table.values()}
    return next(port for port in range(FIRST_PROGRAM_PORT, FIRST_PROGRAM_PORT + 10 * MAX_PROGRAMS) if port not in taken)


def start(name, port):
    if RUNNER == "systemd":
        systemctl("restart", name)
        return
    stop(name)
    folder = program_folder(name)
    environment = {**os.environ, "WARP_SERVE_ADDRESS": PROGRAM_ADDRESS, "WARP_NO_WINDOW": "1"}
    log = open(os.path.join(folder, LOG), "w")
    children[name] = subprocess.Popen([WARP, *WARP_ARGUMENTS, os.path.join(folder, SOURCE), str(port)],
                                      cwd=folder, env=environment, stdout=log, stderr=subprocess.STDOUT)


def stop(name):
    if RUNNER == "systemd":
        systemctl("stop", name)
    elif name in children:
        children.pop(name).kill()


def systemctl(verb, name):
    """the polkit rule (warp-lambda.rules) lets this daemon's user start, stop and clean warp-lambda@ units only"""
    subprocess.run(["systemctl", verb, *(["--what=state"] if verb == "clean" else []), UNIT.format(name)], check=verb == "restart", capture_output=True)


def failure_of(name):
    """what the program said before it stopped serving (its unit writes the same log)"""
    with open(os.path.join(program_folder(name), LOG)) as file:
        lines = [line for line in file.read().splitlines() if line.strip() and not line.startswith("serving ")]
    return "\n".join(lines[-6:]) or "it stopped without a word"


def program_connection(port, timeout):
    """from PROGRAM_ADDRESS too: the program's unit drops every other localhost address, 127.0.0.1 included"""
    return http.client.HTTPConnection(PROGRAM_ADDRESS, port, timeout=timeout, source_address=(PROGRAM_ADDRESS, 0))


def answers(port):
    try:
        connection = program_connection(port, 2)
        connection.request("GET", "/")
        connection.getresponse().read()
        return True
    except OSError:
        return False


def serving(name, port):
    for _ in range(START_TRIES):
        if answers(port):
            return
        if RUNNER == "process" and children[name].poll() is not None:
            break
        time.sleep(START_PAUSE)
    message = failure_of(name)
    stop(name)
    raise Refusal(400, f"the program does not serve: {message}")


def deploy(name, source, who):
    if not NAME_PATTERN.match(name or ""):
        raise Refusal(400, "a name of lower-case letters, digits and dashes")
    if len(source) > MAX_SOURCE_BYTES:
        raise Refusal(413, f"the program is over {MAX_SOURCE_BYTES // 1024} KB")
    with lock:
        table = names()
        entry = table.get(name)
        if entry and entry["owner"] != who["id"]:
            raise Refusal(403, f"{name} belongs to someone else")
        if not entry:
            if len(table) >= MAX_PROGRAMS:
                raise Refusal(507, f"pannous.com hosts {MAX_PROGRAMS} programs already")
            if sum(other["owner"] == who["id"] for other in table.values()) >= MAX_PROGRAMS_PER_USER:
                raise Refusal(403, f"{MAX_PROGRAMS_PER_USER} programs per user: remove one first")
            entry = {"owner": who["id"], "login": who["login"], "port": free_port(table)}
        folder = program_folder(name)
        os.makedirs(folder, exist_ok=True)
        with open(os.path.join(folder, SOURCE), "wb") as file:
            file.write(source)
        with open(os.path.join(folder, "env"), "w") as file:
            file.write(f"PORT={entry['port']}\n")
        entry["deployed"] = int(time.time())
        table[name] = entry
        save(table)
    start(name, entry["port"])
    serving(name, entry["port"])
    return {"url": f"https://{name}.{DOMAIN}", "name": name}


def remove(name, who):
    with lock:
        table = names()
        entry = table.get(name)
        if not entry:
            raise Refusal(404, f"no program {name}")
        if entry["owner"] != who["id"]:
            raise Refusal(403, f"{name} belongs to someone else")
        stop(name)
        if RUNNER == "systemd":
            systemctl("clean", name)  # its files and SQLite go with it: the name's next owner starts empty
        del table[name]
        save(table)
    return {"removed": name}


def name_of_host(host):
    host = (host or "").split(":")[0].lower()
    suffix = "." + DOMAIN
    return host[: -len(suffix)] if host.endswith(suffix) else None


class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    server_version = "warp-lambda"

    def log_message(self, form, *arguments):
        sys.stderr.write(f"{self.address_string()} {form % arguments}\n")

    def cors(self):
        origin = self.headers.get("origin")
        if origin and any(pattern.match(origin) for pattern in ALLOWED_ORIGINS):
            return {"access-control-allow-origin": origin, "vary": "origin", "access-control-allow-headers": "authorization, content-type",
                    "access-control-allow-methods": "POST, DELETE, OPTIONS"}
        return {}

    def reply(self, status, body, content_type="application/json", headers=None):
        data = body if isinstance(body, bytes) else json.dumps(body).encode()
        self.send_response(status)
        for key, value in {"content-type": content_type, "content-length": str(len(data)), **self.cors(), **(headers or {})}.items():
            self.send_header(key, value)
        self.end_headers()
        self.wfile.write(data)

    def body(self):
        return self.rfile.read(int(self.headers.get("content-length") or 0))

    def handle_any(self):
        try:
            name = name_of_host(self.headers.get("host"))
            if name:
                return self.proxy(name)
            url = urllib.parse.urlsplit(self.path)
            query = dict(urllib.parse.parse_qsl(url.query))
            if url.path == f"{NATIVE}/ask":
                return self.reply(200 if name_of_host(query.get("domain")) in names() else 404, {})
            if url.path == f"{NATIVE}/deploy" and self.command == "OPTIONS":
                return self.reply(204, b"")
            if url.path == f"{NATIVE}/deploy" and self.command == "POST":
                source = self.body()
                return self.reply(200, deploy(query.get("name"), source, user(self.headers.get("authorization"))))
            if url.path == f"{NATIVE}/deploy" and self.command == "DELETE":
                return self.reply(200, remove(query.get("name"), user(self.headers.get("authorization"))))
            raise Refusal(404, f"no {url.path} here")
        except Refusal as refusal:
            self.reply(refusal.status, {"error": str(refusal)})

    def proxy(self, name):
        entry = names().get(name)
        if not entry:
            raise Refusal(404, f"no program {name} on {DOMAIN}")
        headers = {key: value for key, value in self.headers.items() if key.lower() not in HOP_HEADERS}
        connection = program_connection(entry["port"], PROXY_TIMEOUT)
        try:
            connection.request(self.command, self.path, body=self.body() or None, headers=headers)
            answer = connection.getresponse()
            data = answer.read()
        except OSError:
            raise Refusal(502, f"{name} does not answer")
        self.send_response(answer.status)
        for key, value in answer.getheaders():
            if key.lower() not in HOP_HEADERS and key.lower() != "content-length":
                self.send_header(key, value)
        self.send_header("content-length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    do_GET = do_POST = do_PUT = do_PATCH = do_DELETE = do_OPTIONS = do_HEAD = handle_any


def restart_all():
    """process runner: the programs come back with the daemon (systemd keeps its units itself)"""
    for name, entry in names().items():
        start(name, entry["port"])


if __name__ == "__main__":
    os.makedirs(PROGRAMS, exist_ok=True)
    if RUNNER == "process":
        restart_all()
    server = http.server.ThreadingHTTPServer(("127.0.0.1", PORT), Handler)
    print(f"warp-lambda on 127.0.0.1:{PORT}, programs at *.{DOMAIN}", flush=True)
    try:
        server.serve_forever()
    finally:
        for child in children.values():
            child.kill()

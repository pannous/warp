#!/usr/bin/env python3
"""The playground's bug button (card bug-report): a comment with the program and its output becomes a board card.

The browser can't hold a GitHub secret, so the report goes through pannous.com:
  serve     on pannous.com (unit warp-bug-report behind Ferron's /warp-bug-report): checks the shared token, keeps
            the report as a JSON file in the state directory and makes its card at once with the GitHub token
            /etc/warp/github-token: an issue of pannous/warp on the board, in Next (express: Now, as urgent as a card
            the user drops there; a comment saying "later": Later); done reports move to done/. The Mac's cron gives the card its key (todo keys).
  pull      on the Mac (cron), the fallback for reports whose card GitHub refused: adds each with `todo add` (into
            Next; express: --urgent, into Now), then acknowledges it
  list, ack <id>…   what pull runs on the server
  install   copies this file to the server, installs the unit and the token (made once, kept in TOKEN_FILE here)
  token     prints the playground address that stores the owner token (no report limit) in this browser
Notes: notes/bug_report.md.
"""
import hmac
import json
import os
import re
import secrets
import subprocess
import sys
import threading
import time
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

SERVER = "pannous.com"
PORT = int(os.environ.get("BUG_REPORT_PORT", 5691))
STATE = Path(os.environ.get("STATE_DIRECTORY", "/var/lib/warp-bug-report"))
DONE = "done"
REMOTE_SCRIPT = "/opt/warp-bug-report/bug_report.py"
REMOTE_TOKEN = "/etc/warp-bug-report.token"  # the shared token the playground carries
REMOTE_OWNER_TOKEN = "/etc/warp-bug-report.owner-token"  # the user's own: no limits
GITHUB_TOKEN = "/etc/warp/github-token"  # the user's fine-grained token: Contents, Issues, Projects
GITHUB_GRAPHQL = "https://api.github.com/graphql"
OWNER = "pannous"
REPOSITORY_NAME = "warp"
BOARD_NUMBER = 1  # https://github.com/users/pannous/projects/1
UNIT_PATH = "/etc/systemd/system/warp-bug-report.service"
TOKEN_FILE = Path("~/.config/warp/bug-report-token")  # on the Mac (expanded there: the server unit has no home)
OWNER_TOKEN_FILE = Path("~/.config/warp/bug-report-owner-token")
REPOSITORY = "/Users/me/dev/angles/warp"
PLAYGROUND = "https://warp.pannous.com/"
TOKEN_PARAMETER = "bug_token"  # playground.js BUG_TOKEN_PARAMETER
MAX_BODY = 64 * 1024
# user, 2026-10-17: with the shared token an address may send REPORTS_PER_HOUR reports an hour; one sending more than
# ATTEMPTS_PER_HOUR within an hour is banned for good (unban: delete its line from BANNED in the state directory and
# restart the unit). The owner token (the user's browser, `bug_report.py token`) has no limit.
REPORTS_PER_HOUR = 10
ATTEMPTS_PER_HOUR = 100
BANNED = "banned.txt"
LIMITS = {"comment": 4000, "code": 12000, "output": 4000, "version": 200, "page": 500}
TITLE_LENGTH = 50  # todo TITLE_MAX
TITLE_MARK = "🐞 "
COLUMNS = {False: "Next", True: "Now"}  # by express
LATER_COLUMN = "Later"  # a comment saying the word "later" (card if-such): not now, nor next
LATER_WORD = re.compile(r"\blater\b", re.IGNORECASE)
BOARD_QUERY = """query($owner: String!, $name: String!, $number: Int!) {
  repository(owner: $owner, name: $name) { id }
  user(login: $owner) { projectV2(number: $number) { id field(name: "Status") {
    ... on ProjectV2SingleSelectField { id options { id name } } } } } }"""
CREATE_ISSUE = """mutation($repository: ID!, $title: String!, $body: String) {
  createIssue(input: {repositoryId: $repository, title: $title, body: $body}) { issue { id url } } }"""
ADD_ITEM = """mutation($project: ID!, $content: ID!) {
  addProjectV2ItemById(input: {projectId: $project, contentId: $content}) { item { id databaseId } } }"""
UPDATE_BODY = """mutation($issue: ID!, $body: String!) { updateIssue(input: {id: $issue, body: $body}) { issue { id } } }"""
BOARD_CARD_URL = f"https://github.com/users/{OWNER}/projects/{BOARD_NUMBER}?pane=issue&itemId=%s"  # as todo's link
SET_COLUMN = """mutation($project: ID!, $item: ID!, $field: ID!, $option: String!) {
  updateProjectV2ItemFieldValue(input: {projectId: $project, itemId: $item, fieldId: $field,
    value: {singleSelectOptionId: $option}}) { projectV2Item { id } } }"""
UNIT = f"""[Unit]
Description=warp playground bug reports (card bug-report, warp scripts/bug_report.py)
After=network-online.target

[Service]
ExecStart=/usr/bin/python3 {REMOTE_SCRIPT} serve
Environment=BUG_REPORT_PORT={PORT}
LoadCredential=token:{REMOTE_TOKEN}
LoadCredential=owner:{REMOTE_OWNER_TOKEN}
LoadCredential=github:{GITHUB_TOKEN}
DynamicUser=yes
StateDirectory=warp-bug-report
Restart=always
RestartSec=5
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
PrivateDevices=yes

[Install]
WantedBy=multi-user.target
"""


# ---- on the server ----------------------------------------------------------------------------------------------

def credential(name):
    return (Path(os.environ["CREDENTIALS_DIRECTORY"]) / name).read_text().strip()


def graphql(query, **variables):
    request = urllib.request.Request(GITHUB_GRAPHQL, json.dumps({"query": query, "variables": variables}).encode(),
                                     {"Authorization": f"Bearer {credential('github')}", "Content-Type": "application/json"})
    with urllib.request.urlopen(request, timeout=30) as answer:
        result = json.load(answer)
    if result.get("errors"):
        raise RuntimeError(result["errors"][0].get("message"))
    return result["data"]


def make_card(report):
    """the report's card on the board: an issue in its column_of; the issue's and the card's urls"""
    title, body = card_of(report)
    board = graphql(BOARD_QUERY, owner=OWNER, name=REPOSITORY_NAME, number=BOARD_NUMBER)
    project, status = board["user"]["projectV2"]["id"], board["user"]["projectV2"]["field"]
    issue = graphql(CREATE_ISSUE, repository=board["repository"]["id"], title=title, body=body)["createIssue"]["issue"]
    item = graphql(ADD_ITEM, project=project, content=issue["id"])["addProjectV2ItemById"]["item"]
    board_card = BOARD_CARD_URL % item["databaseId"]
    graphql(UPDATE_BODY, issue=issue["id"], body=f"{body}\n\nBoard card: {board_card}")
    option = next(option["id"] for option in status["options"] if option["name"] == column_of(report))
    graphql(SET_COLUMN, project=project, item=item["id"], field=status["id"], option=option)
    return {"card": issue["url"], "board": board_card}


def kept_report(fields):
    report = {name: str(fields.get(name) or "")[:limit] for name, limit in LIMITS.items()}
    report["express"] = fields.get("express") is True
    report["received"] = time.strftime("%Y-%m-%d %H:%M:%S")
    return report


class SpamGate:
    """Counts each client's POSTs of the last hour: past REPORTS_PER_HOUR they are refused, past ATTEMPTS_PER_HOUR the
    client is banned, kept in BANNED"""

    def __init__(self, banned_file):
        self.banned_file = banned_file
        self.banned = set(banned_file.read_text().split()) if banned_file.exists() else set()
        self.attempts = {}
        self.lock = threading.Lock()

    def refusal(self, client):
        """why this attempt is refused, else empty"""
        now = time.time()
        with self.lock:
            if client in self.banned:
                return "banned"
            recent = [moment for moment in self.attempts.get(client, []) if now - moment < 3600] + [now]
            self.attempts[client] = recent
            if len(recent) <= REPORTS_PER_HOUR:
                return ""
            if len(recent) <= ATTEMPTS_PER_HOUR:
                return f"at most {REPORTS_PER_HOUR} reports an hour"
            self.banned.add(client)
            del self.attempts[client]
            with self.banned_file.open("a") as banned:
                banned.write(client + "\n")
        print(f"banned {client}: more than {ATTEMPTS_PER_HOUR} reports within an hour", file=sys.stderr, flush=True)
        return "banned"


class ReportHandler(BaseHTTPRequestHandler):
    token = None
    owner_token = None
    gate = None

    def client(self):
        """the sender's address: Ferron proxies from 127.0.0.1 and appends the real one to X-Forwarded-For (its last
        entry; earlier ones are the client's own claim)"""
        forwarded = (self.headers.get("X-Forwarded-For") or "").split(",")[-1].strip()
        return forwarded or self.client_address[0]

    def reply(self, status, body):
        data = json.dumps(body).encode() if status != 204 else b""  # a 204 carries no body
        self.send_response(status)
        self.send_header("Access-Control-Allow-Origin", "*")  # the token, not the origin, is the check
        self.send_header("Access-Control-Allow-Methods", "POST")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_OPTIONS(self):
        self.reply(204, {})

    def do_POST(self):
        client = self.client()
        if client in self.gate.banned:
            return self.reply(429, {"error": "this address is banned"})
        length = int(self.headers.get("Content-Length") or 0)
        if not 0 < length <= MAX_BODY:
            return self.reply(413, {"error": f"a report is at most {MAX_BODY} bytes"})
        try:
            fields = json.loads(self.rfile.read(length))
        except ValueError:
            return self.reply(400, {"error": "the report is no JSON"})
        token = str(fields.get("token") or "") if isinstance(fields, dict) else ""
        if not hmac.compare_digest(token, self.owner_token):
            if refusal := self.gate.refusal(client):
                return self.reply(429, {"error": f"too many reports from this address: {refusal}"})
            if not hmac.compare_digest(token, self.token):
                return self.reply(403, {"error": "wrong token"})
        report = kept_report(fields)
        if not report["comment"].strip():
            return self.reply(400, {"error": "the report has no comment"})
        report_id = time.strftime("%Y%m%d-%H%M%S-") + secrets.token_hex(3)
        # queued only when making the card fails: a report queued while its card was being made got a second card
        # from the Mac's pull (card-made-2)
        try:
            urls = make_card(report)
        except Exception as failure:
            (STATE / f"{report_id}.json").write_text(json.dumps(report))
            print(f"{report_id}: no card ({failure}); queued for the Mac's pull", file=sys.stderr, flush=True)
            return self.reply(200, {"queued": report_id})
        (STATE / DONE / f"{report_id}.json").write_text(json.dumps(report))
        self.reply(200, urls)


def serve():
    ReportHandler.token, ReportHandler.owner_token = credential("token"), credential("owner")
    (STATE / DONE).mkdir(parents=True, exist_ok=True)
    ReportHandler.gate = SpamGate(STATE / BANNED)
    ThreadingHTTPServer(("127.0.0.1", PORT), ReportHandler).serve_forever()


def list_reports():
    print(json.dumps({path.stem: json.loads(path.read_text()) for path in sorted(STATE.glob("*.json"))}))


def acknowledge(report_ids):
    for report_id in report_ids:
        (STATE / f"{report_id}.json").rename(STATE / DONE / f"{report_id}.json")


# ---- on the Mac -------------------------------------------------------------------------------------------------

def remote(*command, **options):
    return subprocess.run(["ssh", SERVER, *command], check=True, text=True, capture_output=True, **options).stdout


def fenced(text, language=""):
    return f"```{language}\n{text.rstrip()}\n```"


def shortened(text, length):
    """text cut at the last whole word that fits"""
    return text if len(text) <= length else text[:length].rsplit(" ", 1)[0] + "…"


def column_of(report):
    """Now for express, Later when the comment says "later", else Next"""
    if not report["express"] and LATER_WORD.search(report["comment"]):
        return LATER_COLUMN
    return COLUMNS[report["express"]]


def card_of(report):
    comment = report["comment"].strip()
    title = TITLE_MARK + shortened(comment.splitlines()[0], TITLE_LENGTH - len(TITLE_MARK))
    # the playground's "include source" unticked sends no program and no output: the card leaves them out
    program = [fenced(report["code"], "warp")] if report["code"].strip() else []
    output = ["Output:", fenced(report["output"])] if report["output"].strip() else []
    body = "\n\n".join([f"(user, bug button {report['received']})", comment, *program, *output,
                        f"{report['version'] or 'unversioned build'} · {report['page']}"])
    return title, body


def pull():
    reports = json.loads(remote(f"python3 {REMOTE_SCRIPT} list"))
    for report_id, report in reports.items():
        title, body = card_of(report)
        column = ["--urgent"] if report["express"] else [column_of(report)]  # the user's card, not an agent's (Soon)
        subprocess.run(["todo", "add", title, *column, "-b", body], cwd=REPOSITORY, check=True)
        remote(f"python3 {REMOTE_SCRIPT} ack {report_id}")
        print(f"{time.strftime('%F %T')} {report_id} → card: {title}")


def local_token(token_file=TOKEN_FILE):
    token_file = token_file.expanduser()
    if not token_file.exists():
        token_file.parent.mkdir(parents=True, exist_ok=True)
        token_file.write_text(secrets.token_urlsafe(24))
        token_file.chmod(0o600)
    return token_file.read_text().strip()


def install():
    remote(f"mkdir -p {Path(REMOTE_SCRIPT).parent} && umask 077 && cat > {REMOTE_TOKEN}", input=local_token())
    remote(f"umask 077 && cat > {REMOTE_OWNER_TOKEN}", input=local_token(OWNER_TOKEN_FILE))
    remote(f"cat > {REMOTE_SCRIPT}", input=Path(__file__).read_text())
    remote(f"cat > {UNIT_PATH} && systemctl daemon-reload && systemctl enable --now warp-bug-report"
           " && systemctl restart warp-bug-report", input=UNIT)
    print(remote("systemctl is-active warp-bug-report").strip())


COMMANDS = {"serve": serve, "list": list_reports, "pull": pull, "install": install,
            "token": lambda: print(f"{PLAYGROUND}?{TOKEN_PARAMETER}={local_token(OWNER_TOKEN_FILE)}")}

if __name__ == "__main__":
    if sys.argv[1:2] == ["ack"]:
        acknowledge(sys.argv[2:])
    elif sys.argv[1:2] and sys.argv[1] in COMMANDS:
        COMMANDS[sys.argv[1]]()
    else:
        sys.exit(__doc__)

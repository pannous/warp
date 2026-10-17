#!/usr/bin/env python3
"""The playground's bug button (card bug-report): a comment with the program and its output becomes a board card.

The browser can't hold a GitHub secret, so the report travels in two hops:
  serve     on pannous.com (unit warp-bug-report behind Ferron's /warp-bug-report): checks the shared token and keeps
            each report as a JSON file in the state directory
  pull      on the Mac (cron): fetches the kept reports over ssh, adds each with `todo add` (into Next; express:
            --urgent, into Now), then acknowledges it, which moves it to done/ on the server
  list, ack <id>…   what pull runs on the server
  install   copies this file to the server, installs the unit and the token (made once, kept in TOKEN_FILE here)
  token     prints the playground address that stores the token in this browser
Notes: notes/bug_report.md.
"""
import hmac
import json
import os
import secrets
import subprocess
import sys
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

SERVER = "pannous.com"
PORT = int(os.environ.get("BUG_REPORT_PORT", 5691))
STATE = Path(os.environ.get("STATE_DIRECTORY", "/var/lib/warp-bug-report"))
DONE = "done"
REMOTE_SCRIPT = "/opt/warp-bug-report/bug_report.py"
REMOTE_TOKEN = "/etc/warp-bug-report.token"
UNIT_PATH = "/etc/systemd/system/warp-bug-report.service"
TOKEN_FILE = Path("~/.config/warp/bug-report-token")  # on the Mac (expanded there: the server unit has no home)
REPOSITORY = "/Users/me/dev/angles/warp"
PLAYGROUND = "https://warp.pannous.com/"
TOKEN_PARAMETER = "bug_token"  # playground.js BUG_TOKEN_PARAMETER
MAX_BODY = 64 * 1024
LIMITS = {"comment": 4000, "code": 12000, "output": 4000, "version": 200, "page": 500}
TITLE_LENGTH = 80
UNIT = f"""[Unit]
Description=warp playground bug reports (card bug-report, warp scripts/bug_report.py)
After=network-online.target

[Service]
ExecStart=/usr/bin/python3 {REMOTE_SCRIPT} serve
Environment=BUG_REPORT_PORT={PORT}
LoadCredential=token:{REMOTE_TOKEN}
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

def server_token():
    return (Path(os.environ["CREDENTIALS_DIRECTORY"]) / "token").read_text().strip()


def kept_report(fields):
    report = {name: str(fields.get(name) or "")[:limit] for name, limit in LIMITS.items()}
    report["express"] = fields.get("express") is True
    report["received"] = time.strftime("%Y-%m-%d %H:%M:%S")
    return report


class ReportHandler(BaseHTTPRequestHandler):
    token = None

    def reply(self, status, body):
        data = json.dumps(body).encode()
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
        length = int(self.headers.get("Content-Length") or 0)
        if not 0 < length <= MAX_BODY:
            return self.reply(413, {"error": f"a report is at most {MAX_BODY} bytes"})
        try:
            fields = json.loads(self.rfile.read(length))
        except ValueError:
            return self.reply(400, {"error": "the report is no JSON"})
        if not isinstance(fields, dict) or not hmac.compare_digest(str(fields.get("token") or ""), self.token):
            return self.reply(403, {"error": "wrong token"})
        report = kept_report(fields)
        if not report["comment"].strip():
            return self.reply(400, {"error": "the report has no comment"})
        report_id = time.strftime("%Y%m%d-%H%M%S-") + secrets.token_hex(3)
        (STATE / f"{report_id}.json").write_text(json.dumps(report))
        self.reply(200, {"queued": report_id})


def serve():
    ReportHandler.token = server_token()
    (STATE / DONE).mkdir(parents=True, exist_ok=True)
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


def card_of(report):
    comment = report["comment"].strip()
    title = "playground: " + comment.splitlines()[0][:TITLE_LENGTH]
    body = "\n\n".join([f"(user, bug button {report['received']})", comment, fenced(report["code"], "warp"),
                        "Output:", fenced(report["output"]), f"{report['version'] or 'unversioned build'} · {report['page']}"])
    return title, body


def pull():
    reports = json.loads(remote(f"python3 {REMOTE_SCRIPT} list"))
    for report_id, report in reports.items():
        title, body = card_of(report)
        column = ["--urgent"] if report["express"] else ["Next"]  # the user's card, not an agent's (Soon)
        subprocess.run(["todo", "add", title, *column, "-b", body], cwd=REPOSITORY, check=True)
        remote(f"python3 {REMOTE_SCRIPT} ack {report_id}")
        print(f"{time.strftime('%F %T')} {report_id} → card: {title}")


def local_token():
    token_file = TOKEN_FILE.expanduser()
    if not token_file.exists():
        token_file.parent.mkdir(parents=True, exist_ok=True)
        token_file.write_text(secrets.token_urlsafe(24))
        token_file.chmod(0o600)
    return token_file.read_text().strip()


def install():
    remote(f"mkdir -p {Path(REMOTE_SCRIPT).parent} && umask 077 && cat > {REMOTE_TOKEN}", input=local_token())
    remote(f"cat > {REMOTE_SCRIPT}", input=Path(__file__).read_text())
    remote(f"cat > {UNIT_PATH} && systemctl daemon-reload && systemctl enable --now warp-bug-report"
           " && systemctl restart warp-bug-report", input=UNIT)
    print(remote("systemctl is-active warp-bug-report").strip())


COMMANDS = {"serve": serve, "list": list_reports, "pull": pull, "install": install,
            "token": lambda: print(f"{PLAYGROUND}?{TOKEN_PARAMETER}={local_token()}")}

if __name__ == "__main__":
    if sys.argv[1:2] == ["ack"]:
        acknowledge(sys.argv[2:])
    elif sys.argv[1:2] and sys.argv[1] in COMMANDS:
        COMMANDS[sys.argv[1]]()
    else:
        sys.exit(__doc__)

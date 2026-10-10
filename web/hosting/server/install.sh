#!/bin/bash
# Installs warp-lambda on pannous.com (notes/hosting.md "Ferron"): run from this folder,
#   web/hosting/server/install.sh [path of a linux x86_64 warp binary]
# copies the daemon, the units, the polkit rule and the binary over ssh and (re)starts the daemon. Idempotent.
# Ferron's side (*.lambda.pannous.com, lambda.pannous.com/native/, the on-demand TLS ask) is in pannous-lockdown.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
SERVER=${WARP_LAMBDA_SERVER:-pannous.com}
WARP_BINARY=${1:-}

scp -q "$HERE/warp_lambda.py" "$HERE/warp-lambda.service" "$HERE/warp-lambda@.service" "$HERE/warp-lambda.rules" "$SERVER:/tmp/"
[ -n "$WARP_BINARY" ] && scp -q "$WARP_BINARY" "$SERVER:/tmp/warp"
ssh "$SERVER" bash -s <<'REMOTE'
set -euo pipefail
id warp-lambda >/dev/null 2>&1 || useradd --system --home-dir /var/lib/warp-lambda --shell /usr/sbin/nologin warp-lambda
install -d -m 0755 /opt/warp-lambda
install -m 0644 /tmp/warp_lambda.py /opt/warp-lambda/warp_lambda.py
install -m 0644 /tmp/warp-lambda.service /tmp/warp-lambda@.service /etc/systemd/system/
install -m 0644 /tmp/warp-lambda.rules /etc/polkit-1/rules.d/50-warp-lambda.rules
[ -f /tmp/warp ] && install -m 0755 /tmp/warp /usr/local/bin/warp
rm -f /tmp/warp_lambda.py /tmp/warp-lambda.service /tmp/warp-lambda@.service /tmp/warp-lambda.rules /tmp/warp
systemctl daemon-reload
systemctl enable --now warp-lambda.service
systemctl restart warp-lambda.service
for _ in $(seq 20); do curl -s -o /dev/null "http://127.0.0.1:8890/native/ask?domain=x" && break; sleep 0.5; done
echo "warp-lambda: $(systemctl is-active warp-lambda.service), warp $(/usr/local/bin/warp --version 2>/dev/null | head -1)"
REMOTE

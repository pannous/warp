#!/bin/bash
# Which scopes the Cloudflare OAuth client "warp hosting" may request, without a login (notes/hosting.md, card
# cf-deploy-access): the authorize page redirects an unregistered scope with error=invalid_scope. Prints the allowed
# ones of the given scopes, or of Cloudflare's whole catalog (GET /client/v4/oauth/scopes, ~/.keys CLOUDFLARE_API_TOKEN).
CLIENT_ID=2ee787da4068961d35487da0f2c5d9ae
AUTHORIZE="https://dash.cloudflare.com/oauth2/auth?client_id=$CLIENT_ID&redirect_uri=https%3A%2F%2Flambda.pannous.com%2Fcallback%2Fcloudflare&response_type=code&state=x&code_challenge=rwcK5i6hFKlknhtk8Oc6M26RNu6YpHEi6T99Hlr3hNI&code_challenge_method=S256"
allowed() {
	curl -s -o /dev/null -w "%{redirect_url}" "$AUTHORIZE&scope=$1" | grep -q invalid_scope || echo "$1"
}
export -f allowed
export AUTHORIZE
if [ $# -gt 0 ]; then printf '%s\n' "$@"; else
	CLOUDFLARE_API_TOKEN=$(source ~/.keys 2>/dev/null; echo "$CLOUDFLARE_API_TOKEN")
	curl -s -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" https://api.cloudflare.com/client/v4/oauth/scopes |
		python3 -c 'import json,sys; [print(s["id"]) for s in json.load(sys.stdin)["result"]]'
	echo offline_access
fi | xargs -P 16 -I{} bash -c 'allowed {}'

#!/usr/bin/env python3
"""One-off helper to obtain a Gmail API refresh token via the OAuth2 flow.

Reads GMAIL_CLIENT_ID / GMAIL_CLIENT_SECRET from backend/.env, runs the consent
flow using a loopback redirect, exchanges the returned code for tokens, and
prints the refresh token (and offers to write it back into backend/.env).

Usage:
    python3 scripts/gmail_auth.py

Requirements in Google Cloud console for the OAuth client:
  - Gmail API enabled.
  - The redirect URI below (http://localhost:8765/) added to the client. For a
    "Desktop app" client, loopback redirects are allowed automatically. For a
    "Web application" client, add it explicitly under Authorized redirect URIs.
  - Your Google account listed as a Test user if the app is in "Testing".
"""

import http.server
import os
import sys
import urllib.parse
import urllib.request
import webbrowser
from pathlib import Path

SCOPE = "https://www.googleapis.com/auth/gmail.readonly"
REDIRECT_PORT = 8765
REDIRECT_URI = f"http://localhost:{REDIRECT_PORT}/"
AUTH_ENDPOINT = "https://accounts.google.com/o/oauth2/v2/auth"
TOKEN_ENDPOINT = "https://oauth2.googleapis.com/token"

ENV_PATH = Path(__file__).resolve().parent.parent / "backend" / ".env"


def read_env(path: Path) -> dict:
    values = {}
    if not path.exists():
        return values
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, _, val = line.partition("=")
        values[key.strip()] = val.strip()
    return values


def capture_code() -> str:
    """Runs a one-shot local server and returns the auth code from the redirect."""
    code_holder = {}

    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):  # noqa: N802
            query = urllib.parse.urlparse(self.path).query
            params = urllib.parse.parse_qs(query)
            code_holder["code"] = params.get("code", [None])[0]
            code_holder["error"] = params.get("error", [None])[0]
            self.send_response(200)
            self.send_header("Content-Type", "text/html")
            self.end_headers()
            msg = "Authorization complete. You can close this tab and return to the terminal."
            self.wfile.write(f"<html><body><h3>{msg}</h3></body></html>".encode())

        def log_message(self, *args):  # silence request logging
            pass

    server = http.server.HTTPServer(("localhost", REDIRECT_PORT), Handler)
    server.handle_request()
    server.server_close()
    if code_holder.get("error"):
        sys.exit(f"Authorization failed: {code_holder['error']}")
    code = code_holder.get("code")
    if not code:
        sys.exit("No authorization code received.")
    return code


def exchange_code(client_id: str, client_secret: str, code: str) -> dict:
    data = urllib.parse.urlencode(
        {
            "code": code,
            "client_id": client_id,
            "client_secret": client_secret,
            "redirect_uri": REDIRECT_URI,
            "grant_type": "authorization_code",
        }
    ).encode()
    req = urllib.request.Request(TOKEN_ENDPOINT, data=data)
    try:
        with urllib.request.urlopen(req) as resp:
            import json

            return json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        sys.exit(f"Token exchange failed ({e.code}): {e.read().decode()}")


def main():
    env = read_env(ENV_PATH)
    client_id = env.get("GMAIL_CLIENT_ID") or os.environ.get("GMAIL_CLIENT_ID", "")
    client_secret = env.get("GMAIL_CLIENT_SECRET") or os.environ.get("GMAIL_CLIENT_SECRET", "")
    if not client_id or not client_secret:
        sys.exit(f"Set GMAIL_CLIENT_ID and GMAIL_CLIENT_SECRET in {ENV_PATH} first.")

    auth_url = AUTH_ENDPOINT + "?" + urllib.parse.urlencode(
        {
            "client_id": client_id,
            "redirect_uri": REDIRECT_URI,
            "response_type": "code",
            "scope": SCOPE,
            "access_type": "offline",
            "prompt": "consent",
        }
    )

    print("\nOpen this URL in your browser and authorize access:\n")
    print(auth_url + "\n")
    try:
        webbrowser.open(auth_url)
    except Exception:
        pass
    print(f"Waiting for the redirect to {REDIRECT_URI} ...\n")

    code = capture_code()
    tokens = exchange_code(client_id, client_secret, code)
    refresh_token = tokens.get("refresh_token")
    if not refresh_token:
        sys.exit(
            "No refresh_token in the response. Revoke prior access at "
            "https://myaccount.google.com/permissions and rerun (prompt=consent is set)."
        )

    print("Refresh token obtained:\n")
    print(refresh_token + "\n")

    answer = input(f"Write GMAIL_REFRESH_TOKEN into {ENV_PATH}? [y/N] ").strip().lower()
    if answer == "y":
        text = ENV_PATH.read_text()
        if "GMAIL_REFRESH_TOKEN=" in text:
            lines = [
                f"GMAIL_REFRESH_TOKEN={refresh_token}"
                if line.startswith("GMAIL_REFRESH_TOKEN=")
                else line
                for line in text.splitlines()
            ]
            ENV_PATH.write_text("\n".join(lines) + "\n")
        else:
            with ENV_PATH.open("a") as f:
                f.write(f"\nGMAIL_REFRESH_TOKEN={refresh_token}\n")
        print(f"Updated {ENV_PATH}.")
    else:
        print("Copy the token into GMAIL_REFRESH_TOKEN in backend/.env yourself.")


if __name__ == "__main__":
    main()

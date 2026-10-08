#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
host="${1:?Usage: scripts/deploy-sync.sh user@host}"
npm run build:sync-web
ssh "$host" 'mkdir -p ~/apps/sparkpad/{crates,apps/sync-web,deploy/sync,data}; chmod 700 ~/apps/sparkpad'
rsync -az --exclude target crates/ "$host:apps/sparkpad/crates/"
rsync -az apps/sync-web/dist/ "$host:apps/sparkpad/apps/sync-web/dist/"
rsync -az deploy/sync/ "$host:apps/sparkpad/deploy/sync/"
scp -q Cargo.lock "$host:apps/sparkpad/Cargo.lock"
ssh "$host" 'cd ~/apps/sparkpad; python3 - <<"PY"
import pathlib,secrets
pathlib.Path("Cargo.toml").write_text("[workspace]\nmembers = [\"crates/sync-core\", \"crates/sync-server\"]\nresolver = \"2\"\n")
p=pathlib.Path(".env")
if not p.exists(): p.write_text("SPARKPAD_SERVER_TOKEN="+secrets.token_urlsafe(48)+"\n")
p.chmod(0o600)
PY
~/.cargo/bin/cargo build --release -p sparkpad-sync-server && docker compose -f deploy/sync/compose.yaml --project-directory . up -d --build'

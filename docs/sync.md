# Local-first sync and self-hosting

Sparkpad has two storage modes. Local mode needs no account, network, or server. Connected mode keeps the same local SQLite database and synchronizes changes in the background. Opening a page, typing, and saving never wait for a network response. Disconnecting keeps downloaded notes on the device.

## Connect a Mac

Open **Storage & sync** using the cloud button in the sidebar footer. Enter your server URL and its connection key. Connecting adds this device's local notes to the shared workspace and downloads the server's notes. Choose **Use local mode** to disconnect.

New installations offer these choices during onboarding. Existing installations keep local mode until you explicitly connect.

The URL and key are stored beside the database in a `*.sync.json` file with owner-only permissions. The key grants access to the entire workspace. Share it only with devices or people who should have that access.

## Access from a phone or another computer

Open the server URL in a browser and enter the same connection key. The web client caches documents in IndexedDB, supports concurrent editing and Markdown preview, and reconnects automatically. The native app keeps its block editor; the initial web client edits Markdown with a preview and interactive task checkboxes.

On HTTPS (or localhost), a service worker also caches the application shell so it can reopen offline. A plain HTTP LAN address can edit offline in an already open tab, but cannot install the service worker. Use HTTPS before exposing a server outside a trusted local network; HTTP does not encrypt the connection key or note content.

## Deploy

Requirements: a Linux host with Rust, Docker, Docker Compose, Python 3, rsync, and SSH access. On your development machine, run `npm ci`, then:

```sh
./scripts/deploy-sync.sh user@your-server
```

The script installs only `~/apps/sparkpad`, creates an owner-only `.env` with a random connection key, builds the Rust backend and the web client, and starts the `sparkpad-sync` container on port **7348**. The service restarts automatically. SQLite and images remain in `~/apps/sparkpad/data` across rebuilds. The runtime runs as UID 1000 with a read-only root filesystem and a 512 MB memory limit.

Read the connection key from `.env` on the server. Do not put it in source control, a URL, or command-line arguments. The public `/health` endpoint reports service health; all note synchronization, session checks, and image endpoints require authentication. The server represents one shared workspace, not a multi-tenant service.

For manual development:

```sh
npm run build:sync-web
export SPARKPAD_SERVER_TOKEN="$(openssl rand -hex 32)"
cargo run -p sparkpad-sync-server
```

The default bind address is `127.0.0.1:7348`. Configure `SPARKPAD_SERVER_BIND`, `SPARKPAD_SERVER_DATA`, and `SPARKPAD_SERVER_WEB` as needed.

## Command line

```sh
sparkpad sync connect http://your-server:7348 # reads the key from stdin
sparkpad sync status
sparkpad sync disconnect
sparkpad sync run                           # optional headless sync worker
```

GUI and MCP processes coordinate using a database-specific file lock, so only one process opens a sync connection. MCP writes to the same SQLite database and enters the same synchronization journal.

## How reconciliation works

Each page has a Yrs/Yjs CRDT document for its title and Markdown body. A separate workspace document contains page hierarchy, groups, ordering, icons, covers, and deletion markers. State vectors exchange missing operations over WebSockets. A durable local outbox retains unacknowledged changes; the server acknowledges updates only after a SQLite commit. Reconnects and repeated updates are safe.

Local undo tracks the editing instance's operations, so it preserves changes received from another device. Remote updates reuse native input entities to preserve focus. Parent cycles created by concurrent moves resolve deterministically. Deletion markers prevent an offline edit from resurrecting a deleted page. Imported images use content-addressed storage and authenticated transfer. Up to 32 recent revision snapshots per page support rebasing stale native/MCP writes; older stale revisions require a reload rather than overwriting newer work.

This first version synchronizes Markdown text, not semantic block objects. Two people can type in the same page concurrently, but competing changes to the same Markdown delimiters or moves of the same paragraph can still produce formatting that needs manual adjustment. Metadata fields resolve concurrent assignments using the CRDT's deterministic ordering. Presence indicators, cursor sharing, per-user permissions, end-to-end encryption, and server history are not implemented.

## Backups and updates

Back up the entire server `data` directory and `.env`. For a consistent SQLite backup while running, use SQLite's backup API or stop the container briefly before copying the directory. Back up the native database and its adjacent `.assets` directory as well. Losing the key requires distributing a new key to every connected device.

Redeploy using the same script to preserve the data directory and key. Server and client protocol versions must agree. Browser data belongs to the server origin; changing the URL creates a separate browser cache.

## Build the macOS installer

```sh
./scripts/installer.sh release
```

This creates an architecture-specific DMG with the application and an Applications shortcut. Community builds are ad-hoc signed. Apple notarization requires an Apple Developer signing identity; the build does not claim to be notarized.

## Verify changes

```sh
cargo test --no-default-features
cargo test -p sparkpad-sync -p sparkpad-sync-server
cargo test --test rich_text                 # macOS native event regressions
cargo build --no-default-features
cargo build -p sparkpad-sync-server
npm run build:sync-web
python3 scripts/verify-sync.py              # real server and two native workers
```

The network regression uses synthetic notes in temporary databases and a random test connection key. It verifies offline reconciliation, server restart, and authentication without connecting the user's workspace.

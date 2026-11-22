# NordTorrent API v1

Base URL: `http://<server>:8080/api/v1`

The API returns JSON. Errors look like this:

```json
{
  "error": {
    "code": "bad_request",
    "message": "The magnet link is invalid."
  }
}
```

## Authentication

The web panel uses an `HttpOnly` cookie set by `/auth/login` or `/auth/setup`.

Android, the Chrome extension and scripts use the bearer token from the login response:

```http
Authorization: Bearer <access_token>
```

State-changing calls made with the cookie must also send:

```http
X-Requested-With: NordTorrent
```

The token is valid for `expires_in` seconds (30 days by default). Changing the password invalidates every older token for that user.

## Endpoints

### Public

#### `GET /status`

```json
{
  "setup_required": false,
  "version": "0.2.0",
  "server_time": "2026-11-14T12:00:00+00:00",
  "api_version": 1,
  "features": ["download-browser", "network-status", "theme-language", "url-routing", "torrent-details", "sessions", "private-trackers", "tracker-completed", "stats-persistence", "torrent-subfolder", "monthly-stats", "thumbnails", "move-torrent", "extension-api", "speed-limits", "seeding-limits", "dht-toggle", "rss", "2fa", "labels", "totp-proxy-detect"]
}
```

`setup_required` is only `true` until the first administrator exists. Tooling can use `features` to verify that a running build contains the expected behaviour.

#### `POST /auth/setup`

Can only be called once, while no users exist.

```json
{
  "username": "jonathan",
  "display_name": "Jonathan",
  "password": "a long sentence of at least 12 characters"
}
```

Responds like login and also sets the cookie.

#### `POST /auth/login`

```json
{
  "username": "jonathan",
  "password": "...",
  "totp_code": "123456"
}
```

`totp_code` is only required when the account has 2FA enabled and the login comes from a non-local IP. Local/private network addresses skip 2FA. The server checks the socket peer and the forwarding headers (`CF-Connecting-IP`, `True-Client-IP`, `X-Real-IP`, `X-Forwarded-For`); if any of them is public, the login counts as remote. This is what makes a Cloudflare Tunnel ask for the code. A missing code returns `401` with `totp_required`; a wrong code returns `401` with `totp_invalid`.

Response:

```json
{
  "user": {
    "username": "jonathan",
    "display_name": "Jonathan",
    "role": "admin",
    "totp_enabled": false,
    "created_at": "2026-11-14T12:00:00+00:00"
  },
  "access_token": "eyJhbGciOiJIUzI1NiIs...",
  "token_type": "Bearer",
  "expires_in": 2592000
}
```

Too many failures: `429` with `rate_limited`.

#### `POST /auth/logout`

Revokes the current server session and clears the web cookie. The bearer token can no longer be used.

### Requires login

#### `GET /auth/me`

Returns the current user.

#### `POST /auth/password`

```json
{
  "current_password": "...",
  "new_password": "another long sentence"
}
```

Returns a new token/cookie and invalidates every older token.

#### `POST /auth/totp/setup`

Generates a new TOTP secret and returns:

```json
{
  "secret": "BASE32...",
  "otpauth_url": "otpauth://totp/...",
  "qr_svg": "<svg ...>"
}
```

`qr_svg` is an inline SVG of the `otpauth://` URL, safe to render directly as a data URI. The secret is only activated after a valid code is confirmed via `/auth/totp/enable`. Calling setup while 2FA is already enabled returns `409`.

#### `POST /auth/totp/enable`

```json
{ "code": "123456" }
```

Enables 2FA for the current account.

#### `POST /auth/totp/disable`

```json
{ "password": "..." }
```

Disables 2FA after confirming with the account password.

#### `GET /auth/sessions` (admin)

Returns every active server session with user, IP, user agent, creation/last-seen time and whether it is the current session.

```json
[
  {
    "id": "…",
    "username": "admin",
    "role": "admin",
    "ip": "192.168.1.10",
    "user_agent": "Mozilla/5.0 …",
    "created_at": "2026-11-15T12:00:00+00:00",
    "last_seen_at": "2026-11-15T12:30:00+00:00",
    "expires_at": 1780000000,
    "current": true
  }
]
```

The web panel labels the Chrome extension sessions from the `X-NordTorrent-Client` header the extension sends on login.

#### `DELETE /auth/sessions/{id}` (admin)

Revokes the session with the given id. If it is the current session the cookie is cleared and the client must sign in again.

#### `GET /users` (admin)

List of users without password hashes.

#### `POST /users` (admin)

```json
{
  "username": "tv",
  "display_name": "Living room",
  "password": "a long sentence",
  "role": "viewer"
}
```

`role` is `admin` or `viewer`.

#### `DELETE /users/{username}` (admin)

Deletes the user. You cannot delete yourself or the last administrator.

### Torrents

#### `GET /download-folders` (admin)

Returns the download folders discovered under `/downloads`, shown by their last path segment.

```json
[
  {
    "name": "complete",
    "path": "complete",
    "default": true
  },
  {
    "name": "series2",
    "path": "series2",
    "default": false
  }
]
```

Folders are discovered from the container mounts. The order is alphabetical and `default` marks the preselected folder. `path` is the relative path to send to the torrent endpoints; the download root itself uses `path: "."`.

#### `GET /download-browser?path=&q=` (admin)

Used by the folder picker in the web panel. Returns one level below the download root:

```json
{
  "path": "series4",
  "display_path": "/downloads/series4",
  "parent": "",
  "entries": [
    {
      "name": "Some Show (2026)",
      "path": "series4/Some Show (2026)"
    }
  ]
}
```

- `path` is relative and may not contain `..`.
- `parent` is `null` at the root, otherwise the parent's relative path.
- `q` filters by folder name in the current level.
- Only folders are listed, never files.

#### `GET /labels`

```json
{
  "labels": ["Movies", "Series"],
  "assignments": { "info-hash": "Movies" },
  "colors": { "Movies": "#fbbf24" }
}
```

#### `POST /labels` (admin)

```json
{ "name": "Movies", "color": "#fbbf24" }
```

Names are unique case-insensitively and limited to 40 characters. `color` is optional and accepts `#rgb` or `#rrggbb` (stored lower case). Labels without a colour are absent from `colors` and keep the neutral default styling.

#### `PUT /labels/{name}` (admin)

```json
{ "color": "#7dd3fc" }
```

Changes the colour of an existing label. Send `{"color": null}` to remove the colour again.

#### `DELETE /labels/{name}` (admin)

Deletes the label and removes it from every torrent.

#### `PUT /torrents/{id}/label` (admin)

```json
{ "label": "Movies" }
```

Set `label` to `null` to remove the label from the torrent. `POST /torrents` and `POST /torrents/upload` also accept an optional `label` field.

#### `GET /torrents`

```json
{
  "torrents": [
    {
      "id": 1,
      "info_hash": "cab507494d02ebb1178b38f2e9d7be299c86b862",
      "name": "Example",
      "state": "downloading",
      "label": "Movies",
      "folder": "series2",
      "output_folder": "/downloads/series2",
      "progress": 42.1,
      "progress_bytes": 421000000,
      "total_bytes": 1000000000,
      "uploaded_bytes": 10000000,
      "download_speed_bps": 5242880.0,
      "upload_speed_bps": 102400.0,
      "eta_seconds": 110,
      "ratio": 0.02,
      "finished": false,
      "error": null,
      "media_files": 2,
      "total_files": 7
    }
  ],
  "totals": {
    "torrent_count": 1,
    "active": 1,
    "paused": 0,
    "completed": 0,
    "download_speed_bps": 5242880.0,
    "upload_speed_bps": 102400.0,
    "downloaded_bytes": 421000000,
    "uploaded_bytes": 10000000
  }
}
```

`state` is one of `initializing`, `downloading`, `seeding`, `paused`, `error`.

#### `GET /torrents/{id}`

`{id}` can be the numeric id or a 40-character info hash. Returns the same fields plus `files`, `trackers` and `peers`:

```json
{
  "id": 1,
  "name": "Example",
  "files": [
    {
      "id": 0,
      "name": "movie.mkv",
      "path": "Example/movie.mkv",
      "length": 900000000,
      "included": true,
      "priority": "normal",
      "progress_bytes": 400000000,
      "progress": 44.4,
      "kind": "video",
      "mime": "video/x-matroska",
      "playable": true,
      "stream_url": "/api/v1/torrents/1/files/0/stream"
    }
  ],
  "trackers": [
    {
      "url": "https://tracker.example/announce",
      "kind": "https",
      "status": "unknown"
    }
  ],
  "peers": [
    {
      "address": "203.0.113.10:51413",
      "state": "Live",
      "downloaded_bytes": 1048576,
      "pieces": 4,
      "errors": 0,
      "connections": 1,
      "incoming": false
    }
  ]
}
```

`kind` is `video`, `audio`, `subtitle` or `other`. `priority` is `normal` or `skip`; librqbit does not expose separate High/Normal levels, so `included=false` means Skip. `trackers` is only included for administrators, because announce URLs can contain private passkeys.

#### `POST /torrents` (admin)

```json
{
  "magnet": "magnet:?xt=urn:btih:...",
  "paused": false,
  "folder": "series2/Some Show (2026)",
  "label": "Movies"
}
```

`folder` is optional and must be a relative path from `/download-browser` (or `path` from `/download-folders`). If omitted, the default folder is used. `magnet` can also be a 40-character hex info hash. HTTP(S) URLs are not accepted here, which avoids SSRF.

Private torrents use the announce URLs embedded in the torrent. Private info hashes are never sent to the session's public fallback trackers, and DHT/LSD are disabled for private torrents.

#### `POST /torrents/upload` (admin)

`multipart/form-data` with:

- `file`: `.torrent` file, max 20 MB
- `paused`: optional `"true"`/`"false"`
- `folder`: optional relative path from `/download-browser`
- `label`: optional label name

#### `POST /torrents/{id}/pause` (admin)

#### `POST /torrents/{id}/resume` (admin)

#### `DELETE /torrents/{id}?delete_files=true` (admin)

`delete_files` defaults to `false`, so only the torrent entry is removed.

#### `POST /torrents/{id}/move` (admin)

Moves a torrent to another folder under the download root. The torrent is paused, forgotten by the engine, the files are moved, and the torrent is added again on the new path and resumed (unless it was paused before).

```json
{
  "folder": "series3"
}
```

`folder` is a relative path from `/download-browser`. A multi-file torrent automatically gets its own torrent-name folder inside the chosen folder.

#### `POST /torrents/{id}/files` (admin)

Selects which files to download. The list contains file ids from the detail response.

```json
{
  "included": [0, 2]
}
```

#### `GET /torrents/{id}/files/{file_id}/stream`

Streams the file with `Range` support. Requires the cookie or a bearer token.

Range example:

```http
Range: bytes=1048576-
```

Responses:

- `200 OK` with `Accept-Ranges: bytes`
- `206 Partial Content` with `Content-Range`
- `416 Range Not Satisfiable` for an invalid range

Android TV/ExoPlayer must send the `Authorization` header on every stream call. Browsers use the cookie automatically.

### Library

#### `GET /library`

Returns every playable video and audio file from all torrents, including files that are still downloading.

```json
{
  "items": [
    {
      "torrent_id": 1,
      "file_id": 0,
      "info_hash": "...",
      "title": "Example",
      "file_name": "movie.mkv",
      "torrent_name": "Example",
      "size": 900000000,
      "downloaded_bytes": 400000000,
      "progress": 44.4,
      "kind": "video",
      "mime": "video/x-matroska",
      "stream_url": "/api/v1/torrents/1/files/0/stream",
      "preview_url": "/api/v1/torrents/1/files/0/thumbnail",
      "torrent_state": "downloading"
    }
  ]
}
```

#### `GET /torrents/{id}/files/{file_id}/thumbnail`

Returns a JPEG preview generated with `ffmpeg` and cached in `/data/thumbnails/`. Used by the TV library. Requires the cookie or a bearer token.

### Network

#### `GET /network`

Returns the local torrent port, the configured/discovered public endpoint and a best-effort port status.

```json
{
  "listen_port": 42000,
  "port_start": 42000,
  "port_end": 42009,
  "configured_external_ip": null,
  "configured_external_port": 50000,
  "external_ip": "203.0.113.10",
  "external_port": 50000,
  "external_endpoint": "203.0.113.10:50000",
  "ip_source": "auto",
  "status": "open",
  "message": "The port answered from the public IP. Other peers should be able to connect.",
  "checked_at": "2026-11-15T11:00:00Z",
  "public_url": "https://torrent.example.com"
}
```

`status` is `open`, `closed` or `unknown`. The check works by having the server open a TCP connection to the public IP/port itself. Behind NAT, `unknown` can mean the router does not allow NAT loopback even though port forwarding works.

#### `POST /network/check`

Forces a new IP lookup and port check. Same response as `GET /network`. Requires login.

#### `PUT /network` (admin)

```json
{
  "external_ip": "203.0.113.10",
  "external_port": 50000
}
```

Set `external_ip` to `null` for automatic discovery. Set `external_port` to `null` to use the local torrent port. Settings are stored in `/data/network.json` and take precedence over `NORDTORRENT_EXTERNAL_IP`/`NORDTORRENT_EXTERNAL_PORT` on the next start.

### Speed limits

#### `GET /limits` (admin)

```json
{
  "download_enabled": true,
  "download_kbps": 1000,
  "upload_enabled": true,
  "upload_kbps": 200
}
```

#### `PUT /limits` (admin)

Same fields as above. Limits apply to all torrents together and are stored in `/data/speed-limits.json`, so they survive restarts. Set `download_enabled`/`upload_enabled` to `false` to remove that limit.

### Seeding

#### `GET /seeding` (admin)

```json
{
  "keep_seeding": true,
  "stop_ratio": 1.0,
  "stop_minutes": 0
}
```

#### `PUT /seeding` (admin)

`keep_seeding=false` stops seeding when a torrent finishes. `stop_ratio` and `stop_minutes` are optional limits; `0` means no limit. A monitor pauses the torrent when a limit is reached. Stored in `/data/seeding.json`.

### DHT

#### `GET /dht` (admin)

```json
{ "enabled": true }
```

#### `PUT /dht` (admin)

Enables or disables DHT. Stored in `/data/dht.json` and applied after the server restarts.

### RSS

#### `GET /rss` (admin)

Returns all RSS feeds.

#### `POST /rss` (admin)

```json
{
  "name": "My feed",
  "url": "https://example.test/rss",
  "folder": "series2",
  "enabled": true,
  "auto_download": true,
  "paused": false,
  "include": "show, s01",
  "exclude": "cam, sample"
}
```

`include` and `exclude` are comma-separated, case-insensitive filters. New matching items are added automatically when the feed is checked.

#### `PUT /rss/{id}` (admin)

Same fields as `POST /rss`.

#### `DELETE /rss/{id}` (admin)

Deletes the feed.

#### `POST /rss/{id}/check` (admin)

Fetches the feed immediately and returns the updated feed. The server checks active feeds every 15 minutes.

### System

#### `GET /system`

```json
{
  "version": "0.2.0",
  "uptime_seconds": 3600,
  "server_time": "2026-11-14T13:00:00+00:00",
  "torrents": { "torrents": [], "totals": { "...": "..." } },
  "torrent_port_start": 42000,
  "torrent_port_end": 42009,
  "user_count": 2,
  "paths": {
    "data_dir": "/data",
    "download_dir": "/downloads"
  }
}
```

`paths` is only returned to administrators.

#### `GET /stats`

Returns monthly and accumulated statistics plus the most uploaded torrents this month.

```json
{
  "month": "2026-11",
  "downloaded_bytes": 2500000000,
  "uploaded_bytes": 7000000000,
  "ratio": 2.8,
  "torrent_count": 4,
  "seeding_count": 2,
  "all_time_downloaded_bytes": 5000000000,
  "all_time_uploaded_bytes": 8000000000,
  "months": [
    { "month": "2026-11", "downloaded_bytes": 2500000000, "uploaded_bytes": 7000000000 }
  ],
  "top_torrents": [
    {
      "info_hash": "…",
      "name": "Example",
      "uploaded_bytes": 6000000000,
      "downloaded_bytes": 2500000000,
      "ratio": 2.4
    }
  ]
}
```

## Android login flow

1. `GET /status`
   - `setup_required=true` -> ask the user to open the web panel.
2. `POST /auth/login` with username and password.
3. Store `access_token` securely (Android Keystore).
4. `GET /library` and `GET /torrents`.
5. Start `PlayerActivity` with `baseUrl + stream_url`.
6. Set `Authorization: Bearer <token>` on ExoPlayer's `DefaultHttpDataSource`.
7. On `401`: clear the token and send the user back to login.

## curl examples

```bash
BASE=http://192.168.1.101:8080

# Status
curl -s "$BASE/api/v1/status"

# Login and store the token
TOKEN=$(curl -s -H 'Content-Type: application/json' \
  -d '{"username":"admin","password":"..."}' \
  "$BASE/api/v1/auth/login" | jq -r .access_token)

# List torrents
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/api/v1/torrents"

# Stream the first 1 MB of a file
curl -H "Authorization: Bearer $TOKEN" \
  -H 'Range: bytes=0-1048575' \
  "$BASE/api/v1/torrents/1/files/0/stream" --output preview.bin
```

## HTTP status codes

| Code | Meaning |
| --- | --- |
| `200` | OK |
| `400` | Invalid request |
| `401` | Not signed in, invalid token, or a required 2FA code |
| `403` | Not allowed (role, Origin or CSRF) |
| `404` | Not found |
| `409` | Conflict (setup already done, label exists) |
| `416` | Invalid Range |
| `429` | Rate limited |
| `5xx` | Server error - check the logs |

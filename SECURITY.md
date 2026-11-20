# Security in NordTorrent

NordTorrent is designed to run on a private network. The focus is on keeping the API unusable without a valid account, making the web panel resistant to CSRF, and keeping the Android token encrypted on the TV. Protecting the server, the network and your media is still your responsibility.

## Threat model

In scope:

- Unauthorized access to the web panel/API from the network.
- Stolen or leaked session tokens.
- CSRF and clickjacking against the web panel.
- Password brute force.
- Accidental exposure of files or tokens in logs.
- A `viewer` account trying to modify torrents.

Out of scope:

- A compromised Proxmox host or Docker daemon.
- A malicious administrator (admins may add torrents and choose files).
- Confidentiality of BitTorrent traffic towards peers/ISP.
- Malicious media codecs in the browser/TV.
- Physical access to the server.

## Controls in the code

### Passwords

- Argon2id via the `argon2` crate with default parameters and a random salt.
- Minimum 12 characters, maximum 256 characters.
- Passwords are never logged or returned.
- Password hashing runs on a blocking worker so the API stays responsive.

### Sessions

- HMAC-SHA256 signed token with `iss`, `sub`, `role`, `ver`, `iat`, `exp` and `jti`.
- A 32-byte random secret is stored in `data/session-secret`.
- The web panel receives an `HttpOnly`, `SameSite=Strict` cookie. Set `NORDTORRENT_COOKIE_SECURE=true` behind HTTPS.
- Android receives a bearer token and stores it encrypted with AES-GCM in Android Keystore.
- Changing the password increments the user's `token_version`, which invalidates every older token.

### CSRF and Origin

- Every state-changing API call made with the cookie requires the header `X-Requested-With: NordTorrent`. A plain HTML form POST cannot set it.
- `SameSite=Strict` prevents the cookie from being sent on cross-site requests.
- `Origin`/`Sec-Fetch-Site` is validated for every non-GET call under `/api/`.
- The API only accepts JSON for JSON endpoints; `multipart/form-data` is used only for `.torrent` uploads.

### Roles

| Action | Admin | Viewer |
| --- | --- | --- |
| View torrents and library | ✅ | ✅ |
| Stream media files | ✅ | ✅ |
| Add/remove/pause torrents | ✅ | ❌ |
| Select files | ✅ | ❌ |
| Create/delete users | ✅ | ❌ |
| See server paths | ✅ | ❌ |

The first account is always an admin. The last admin cannot be deleted, and an admin cannot delete themselves.

### Rate limiting

- Login attempts are limited to 8 failures per 5 minutes per IP+username.
- Unknown users get the same response and a constant delay to make username enumeration harder.
- Behind a reverse proxy, set `NORDTORRENT_TRUST_PROXY=true` before `X-Forwarded-For` is used.

### 2FA and proxied logins

TOTP is skipped only when the request is genuinely local. The server looks at every address it can see: the socket peer, `CF-Connecting-IP` (Cloudflare Tunnel), `True-Client-IP`, `X-Real-IP` and `X-Forwarded-For`. If any of them is a public address, the login is treated as remote and the code is required.

That is what makes a Cloudflare Tunnel work: `cloudflared` connects from loopback or a LAN address, so the socket alone looks local, but the forwarded public address marks the request as remote. Conversely, a LAN browser talking directly to the server still skips 2FA.

Because of this, a LAN client that reaches the panel through a tunnel or reverse proxy is also asked for a code. That is intentional: the proxy is the public entry point. Connect directly to the LAN address if you want the local bypass.

### Input and files

- JSON bodies are limited to 2 MB; `.torrent` uploads to 20 MB.
- The API only accepts magnet links or 40-character info hashes for URL-based adds, so it cannot be used as an arbitrary HTTP client/SSRF.
- File names and torrent names are rendered as text in the UI, not as HTML.
- Streaming validates that the file exists, is selected and belongs to the torrent.
- Streaming supports `Range`, but not arbitrary paths.

### Web security

- CSP with `script-src 'self'`, `object-src 'none'`, `frame-ancestors 'none'`.
- `X-Frame-Options: DENY`, `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`.
- No external CDNs, analytics or third-party scripts.
- No CORS headers; Android uses native HTTP and does not depend on browser CORS. The Chrome extension is allowed explicitly for its own origin.

### Container

- Runs as UID/GID 1000, not root.
- `read_only: true` on the root filesystem in compose, with `tmpfs` for `/tmp`.
- `cap_drop: ALL` and `no-new-privileges:true`.
- Healthcheck against `/healthz`.

## Operational requirements

1. **Use HTTPS.** Put Caddy, Traefik or Nginx Proxy Manager in front and set:
   ```env
   NORDTORRENT_PUBLIC_URL=https://torrent.example.com
   NORDTORRENT_COOKIE_SECURE=true
   NORDTORRENT_TRUST_PROXY=true
   ```
2. **Do not expose port 8080 to the internet.** Use a firewall/VPN. The web port should only be reachable from the LAN/VPN or through the proxy.
3. **Use a viewer account for the TV.** Keep the admin account for administration only.
4. **Remove the bootstrap password** from `.env` once the administrator exists.
5. **Back up `data/`.** `users.json` and `session-secret` are required to keep accounts and sessions.
6. **Update regularly.** `docker compose build --pull` and restart.
7. **Protect the download folder.** Other services on the server should not be able to read/write it freely.

## Known limitations

- Logout clears the web cookie, but an already issued bearer token stays valid until it expires. Change the password to invalidate all tokens.
- TOTP two-factor authentication is available per account. Accounts without it rely on the password alone, so pick a strong one.
- BitTorrent traffic is not encrypted/obfuscated by NordTorrent. Your IP and torrent activity may be visible to trackers, DHT and peers.
- There is no built-in malware scanning. Only download from sources you trust.
- The web player depends on browser codec support; Android TV uses ExoPlayer and supports more formats.
- `.torrent` files can contain web seeds and trackers. Only administrators can upload them.
- Admin is a trusted role. Do not give admin to users you do not trust.

## Reporting

This is a private hobby project. If you find a security issue, please do not open a public issue with sensitive details. Contact the owner of the installation directly.

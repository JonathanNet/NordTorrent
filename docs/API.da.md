# NordTorrent API v1

Base URL: `http://<server>:8080/api/v1`

API'et returnerer JSON. Fejl har formen:

```json
{
  "error": {
    "code": "bad_request",
    "message": "Magnet-linket er ugyldigt."
  }
}
```

## Autentificering

Webpanelet bruger en `HttpOnly`-cookie sat af `/auth/login` eller `/auth/setup`.

Android og scripts bruger bearer-tokenet fra login-svaret:

```http
Authorization: Bearer <access_token>
```

Ændrende kald med cookie skal desuden have:

```http
X-Requested-With: NordTorrent
```

Tokenet er gyldigt i `expires_in` sekunder (standard 30 dage). Et password-skift ugyldiggør alle ældre tokens for brugeren.

## Endpoints

### Offentlige

#### `GET /status`

```json
{
  "setup_required": true,
  "version": "0.4.0",
  "server_time": "2026-11-14T12:00:00+00:00",
  "api_version": 1,
  "features": ["download-browser", "network-status", "theme-language", "url-routing", "torrent-details", "sessions", "private-trackers", "tracker-completed", "stats-persistence", "torrent-subfolder", "monthly-stats", "thumbnails", "move-torrent", "extension-api", "speed-limits", "seeding-limits", "dht-toggle", "rss", "2fa", "labels", "totp-proxy-detect"]
}
```

`setup_required` er kun `true`, indtil den første administrator er oprettet. `features` kan bruges af deploy/testværktøjer til at kontrollere, at den kørende version indeholder de forventede funktioner.

#### `POST /auth/setup`

Kan kun kaldes én gang, når der ikke findes brugere.

```json
{
  "username": "jonathan",
  "display_name": "Jonathan",
  "password": "en lang sætning på mindst 12 tegn"
}
```

Svarer som login. Sætter også cookie.

#### `POST /auth/login`

```json
{
  "username": "jonathan",
  "password": "...",
  "totp_code": "123456"
}
```

`totp_code` er kun nødvendig, når kontoen har 2FA slået til og login
kommer fra en ikke-lokal IP. Lokale/private netværksadresser springer 2FA
over. Serveren kigger både på socket-adressen og på forwarding-headere
(`CF-Connecting-IP`, `True-Client-IP`, `X-Real-IP`, `X-Forwarded-For`); er blot
én af dem offentlig, regnes login som eksternt. Det er derfor Cloudflare Tunnel
nu beder om koden. Ved manglende kode svares `401` med `totp_required`.

Svar:

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

Ved for mange fejl: `429` med `rate_limited`.

#### `POST /auth/logout`

Tilbagekalder den aktuelle server-session og sletter webcookien. Bearer-tokenet kan ikke længere bruges.

### Kræver login

#### `GET /auth/me`

Returnerer den aktuelle bruger.

#### `POST /auth/password`

```json
{
  "current_password": "...",
  "new_password": "en ny lang sætning"
}
```

Returnerer en ny token/cookie og ugyldiggør alle ældre tokens.

#### `POST /auth/totp/setup`

Genererer en ny TOTP-hemmelighed og returnerer:

```json
{
  "secret": "BASE32...",
  "otpauth_url": "otpauth://totp/..."
}
```

Hemmeligheden aktiveres først efter en gyldig kode via `/auth/totp/enable`.

#### `POST /auth/totp/enable`

```json
{ "code": "123456" }
```

Aktiverer 2FA for den aktuelle konto.

#### `POST /auth/totp/disable`

```json
{ "password": "..." }
```

Slår 2FA fra efter bekræftelse med adgangskoden.

#### `GET /auth/sessions` (admin)

Returnerer alle aktive server-sessioner med bruger, IP, user-agent, oprettelses-/seneste aktivitetstid og om det er den aktuelle session.

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

#### `DELETE /auth/sessions/{id}` (admin)

Tilbagekalder sessionen med det angivne id. Hvis det er den aktuelle session, ryddes cookien også, og klienten skal logge ind igen.

#### `GET /users` (admin)

Liste af brugere uden password-hashes.

#### `POST /users` (admin)

```json
{
  "username": "tv",
  "display_name": "Stuen",
  "password": "en lang sætning",
  "role": "viewer"
}
```

`role` er `admin` eller `viewer`.

#### `DELETE /users/{username}` (admin)

Sletter brugeren. Man kan ikke slette sig selv eller den sidste admin.

### Torrents

#### `GET /download-folders` (admin)

Returnerer de downloadmapper, som serveren har fundet under `/downloads`. De vises med deres sidste mappenavn, fx `series2` eller `gamle_serier`.

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

Mapperne opdages automatisk fra containerens mounts. Rækkefølgen er alfabetisk, og `default` markerer den forvalgte mappe. `path` er den relative sti, som skal sendes til torrent-endpointet. Downloadroden selv har `path: "."`.

#### `GET /download-browser?path=&q=` (admin)

Bruges af webpanelets mappe-vælger. Returnerer ét niveau under downloadroden:

```json
{
  "path": "series4",
  "display_path": "/downloads/series4",
  "parent": "",
  "entries": [
    {
      "name": "Carrie (2026)",
      "path": "series4/Carrie (2026)"
    }
  ]
}
```

- `path` er relativ og må ikke indeholde `..`.
- `parent` er `null` i roden, ellers forældremappens relative sti.
- `q` filtrerer på mappenavn i det aktuelle niveau.
- Der vises kun mapper, ikke filer.

#### `GET /torrents`

```json
{
  "torrents": [
    {
      "id": 1,
      "info_hash": "cab507494d02ebb1178b38f2e9d7be299c86b862",
      "name": "Eksempel",
      "state": "downloading",
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

`state` er en af `initializing`, `downloading`, `seeding`, `paused`, `error`.

#### `GET /torrents/{id}`

`{id}` kan være det numeriske id eller en 40-tegns info-hash. Returnerer samme felter plus `files`, `trackers` og `peers`:

```json
{
  "id": 1,
  "name": "Eksempel",
  "files": [
    {
      "id": 0,
      "name": "film.mkv",
      "path": "Eksempel/film.mkv",
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

`kind` er `video`, `audio`, `subtitle` eller `other`. `priority` er `normal` eller `skip`; librqbit eksponerer ikke separate High/Normal-niveauer, så `included=false` er Spring over. `trackers` er kun med for administratorer, fordi announce-URL'er kan indeholde private passkeys.

### Labels

#### `GET /labels`

```json
{
  "labels": ["Film", "Serier"],
  "assignments": { "info-hash": "Film" },
  "colors": { "Film": "#fbbf24" }
}
```

#### `POST /labels` (admin)

```json
{ "name": "Film", "color": "#fbbf24" }
```

`color` er valgfri og accepterer `#rgb` eller `#rrggbb` (gemmes med små bogstaver). Labels uden farve optræder ikke i `colors` og beholder den neutrale standardstil.

#### `PUT /labels/{name}` (admin)

```json
{ "color": "#7dd3fc" }
```

Ændrer farven på en eksisterende label. Send `{"color": null}` for at fjerne farven igen.

#### `DELETE /labels/{name}` (admin)

Sletter labelen og fjerner den fra alle torrents.

#### `PUT /torrents/{id}/label` (admin)

```json
{ "label": "Film" }
```

Sæt `label` til `null` for at fjerne labelen fra torrenten. `POST /torrents`
og `POST /torrents/upload` accepterer også et valgfrit `label`-felt.

#### `POST /torrents` (admin)

```json
{
  "magnet": "magnet:?xt=urn:btih:...",
  "paused": false,
  "folder": "series2/Carrie (2026)"
}
```

`folder` er valgfri og skal være en relativ sti fra `/download-browser` (eller `path` fra `/download-folders`). Udelades den, bruges standardmappen. `magnet` kan også være en 40-tegns hex info-hash. HTTP(S)-URL'er accepteres ikke via dette endpoint (undgår SSRF).

Private torrents bruger alle announce-URL'er fra torrentens tracker-liste. Private info-hashes sendes aldrig til sessionens offentlige fallback-trackere, og DHT/LSD er deaktiveret for private torrents.

#### `POST /torrents/upload` (admin)

`multipart/form-data` med:

- `file`: `.torrent`-fil, maks. 20 MB
- `paused`: valgfri `"true"`/`"false"`
- `folder`: valgfri relativ sti fra `/download-browser`

#### `POST /torrents/{id}/pause` (admin)

#### `POST /torrents/{id}/resume` (admin)

#### `DELETE /torrents/{id}?delete_files=true` (admin)

`delete_files` er som standard `false`, så kun torrent-posten fjernes.

#### `POST /torrents/{id}/move` (admin)

Flytter en torrent til en anden mappe under downloadroden. Torrenten pauses, frigives fra motoren, filerne flyttes, og torrenten genindlæses på den nye placering og startes igen.

```json
{
  "folder": "series3"
}
```

`folder` er en relativ sti fra `/download-browser`. En multi-fil torrent får automatisk sin egen torrent-navn-mappe under den valgte mappe.

#### `POST /torrents/{id}/files` (admin)

Vælger hvilke filer der skal downloades. Listen er fil-id'er fra detalje-svaret.

```json
{
  "included": [0, 2]
}
```

#### `GET /torrents/{id}/files/{file_id}/stream`

Streamer filen med `Range`-support. Kræver cookie eller Bearer-token.

Eksempel på range:

```http
Range: bytes=1048576-
```

Svar:

- `200 OK` med `Accept-Ranges: bytes`
- `206 Partial Content` med `Content-Range`
- `416 Range Not Satisfiable` ved ugyldigt interval

Android TV/ExoPlayer skal sende `Authorization`-headeren på alle stream-kald. Browseren bruger cookien automatisk.

### Bibliotek

#### `GET /library`

Returnerer alle spillbare video- og lydfiler fra torrents, også dem der stadig downloader.

```json
{
  "items": [
    {
      "torrent_id": 1,
      "file_id": 0,
      "info_hash": "...",
      "title": "Eksempel",
      "file_name": "film.mkv",
      "torrent_name": "Eksempel",
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

Returnerer et JPEG-preview, som genereres med `ffmpeg` og caches i `/data/thumbnails/`. Bruges af TV-biblioteket. Kræver cookie eller Bearer-token.

### Netværk

#### `GET /network`

Returnerer den lokale torrent-port, det konfigurerede/auto-opdagede offentlige endpoint og en best-effort portstatus.

```json
{
  "listen_port": 42000,
  "port_start": 42000,
  "port_end": 42009,
  "configured_external_ip": null,
  "configured_external_port": 50000,
  "external_ip": "213.80.120.109",
  "external_port": 50000,
  "external_endpoint": "213.80.120.109:50000",
  "ip_source": "auto",
  "status": "open",
  "message": "Porten svarede fra den offentlige IP. Andre bør kunne forbinde.",
  "checked_at": "2026-11-15T11:00:00Z",
  "public_url": "https://torrent.example.dk"
}
```

`status` er `open`, `closed` eller `unknown`. Tjekket laves ved at serveren selv åbner en TCP-forbindelse til den offentlige IP/port. Bag NAT kan `unknown` betyde, at routeren ikke tillader NAT-loopback, selv om portforwarding virker.

#### `POST /network/check`

Tvinger et nyt IP-opslag og porttjek. Samme svar som `GET /network`. Kræver login.

#### `PUT /network` (admin)

```json
{
  "external_ip": "213.80.120.109",
  "external_port": 50000
}
```

Sæt `external_ip` til `null` for auto-opdagelse. Sæt `external_port` til `null` for at bruge den lokale torrent-port. Indstillingerne gemmes i `/data/network.json` og har forrang over `NORDTORRENT_EXTERNAL_IP`/`NORDTORRENT_EXTERNAL_PORT` ved næste start.

### Hastighedsgrænser

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

Samme felter som ovenfor. Grænserne gælder samlet for alle torrents og gemmes i
`/data/speed-limits.json`, så de også er aktive efter en genstart. Sæt
`download_enabled`/`upload_enabled` til `false` for at fjerne den pågældende
grænse.

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

`keep_seeding=false` stopper seeding, når en torrent er færdig. `stop_ratio`
og `stop_minutes` er valgfrie grænser; `0` betyder ingen grænse. En monitor
pauser torrenten, når en grænse nås. Indstillingerne gemmes i
`/data/seeding.json`.

### DHT

#### `GET /dht` (admin)

```json
{ "enabled": true }
```

#### `PUT /dht` (admin)

Slår DHT til eller fra. Ændringen gemmes i `/data/dht.json` og træder i kraft
efter genstart af serveren.

### I2P (eksperimentel)

#### `GET /i2p` (admin)

```json
{
  "enabled": false,
  "host": "127.0.0.1",
  "port": 7656,
  "mixed_mode": false,
  "pex": true,
  "socks_proxy": "127.0.0.1:4447"
}
```

#### `PUT /i2p` (admin)

Samme felter. `host`/`port` peger på I2P-routerens SAM-bro; `socks_proxy` er routerens SOCKS5-endpoint. Når `enabled` er sand, sendes tracker- og peer-trafik gennem SOCKS-proxyen, og med `mixed_mode: false` slås DHT fra, så klienten ikke annoncerer sig på clearnet. Gemmes i `/data/i2p.json` og læses når torrent-sessionen starter, så en genstart anvender ændringerne.

#### `POST /i2p/test` (admin)

Tester de indsendte indstillinger uden at gemme dem: forbinder til SAM-broen, laver et `HELLO VERSION`-håndtryk, opretter en midlertidig STREAM-session og tjekker om SOCKS-proxyen accepterer en TCP-forbindelse.

```json
{
  "reachable": true,
  "reason": "ok",
  "sam_version": "3.3",
  "destination": "joyg7dsohj3rluqb2vz5bkscg5rokxnl2yncyarhr6swzrwsstqa.b32.i2p",
  "socks_reachable": true
}
```

`reason` er `ok`, `connect_failed`, `timeout`, `hello_failed` eller `session_failed`; `detail` indeholder routerens rå svar, når der er et. Destinationen er kun med, når routeren returnerer den for den midlertidige session.

### RSS

#### `GET /rss` (admin)

Returnerer alle RSS-feeds.

#### `POST /rss` (admin)

```json
{
  "name": "Mit feed",
  "url": "https://example.test/rss",
  "folder": "series2",
  "enabled": true,
  "auto_download": true,
  "paused": false,
  "include": "show, s01",
  "exclude": "cam, sample"
}
```

`include` og `exclude` er kommaseparerede, case-insensitive filtre. Nye
matching elementer tilføjes automatisk, når feedet tjekkes.

#### `PUT /rss/{id}` (admin)

Samme felter som `POST /rss`.

#### `DELETE /rss/{id}` (admin)

Sletter feedet.

#### `POST /rss/{id}/check` (admin)

Henter feedet med det samme og returnerer det opdaterede feed. Serveren tjekker
aktive feeds automatisk hvert 15. minut.

### System

#### `GET /system`

```json
{
  "version": "0.4.0",
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

`paths` vises kun for administratorer.

#### `GET /stats`

Returnerer månedlig og akkumuleret statistik samt de mest uploadede torrents denne måned.

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
      "name": "Eksempel",
      "uploaded_bytes": 6000000000,
      "downloaded_bytes": 2500000000,
      "ratio": 2.4
    }
  ]
}
```

## Android-loginflow

1. `GET /status`
   - `setup_required=true` → bed brugeren åbne webpanelet.
2. `POST /auth/login` med brugernavn og adgangskode.
3. Gem `access_token` sikkert (Android Keystore).
4. `GET /library` og `GET /torrents`.
5. Start `PlayerActivity` med `baseUrl + stream_url`.
6. Sæt `Authorization: Bearer <token>` på ExoPlayerens `DefaultHttpDataSource`.
7. Ved `401`: ryd token og send brugeren til login.

## Eksempler med curl

```bash
BASE=http://192.168.1.101:8080

# Status
curl -s "$BASE/api/v1/status"

# Login og gem token
TOKEN=$(curl -s -H 'Content-Type: application/json' \
  -d '{"username":"admin","password":"..."}' \
  "$BASE/api/v1/auth/login" | jq -r .access_token)

# Liste torrents
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/api/v1/torrents"

# Stream første 1 MB af en fil
curl -H "Authorization: Bearer $TOKEN" \
  -H 'Range: bytes=0-1048575' \
  "$BASE/api/v1/torrents/1/files/0/stream" --output preview.bin
```

## HTTP-statuskoder

| Kode | Betydning |
| --- | --- |
| `200` | OK |
| `206` | Delvist indhold (Range) |
| `400` | Ugyldig anmodning |
| `401` | Ikke logget ind / udløbet token |
| `403` | Rollen må ikke udføre handlingen, eller Origin/CSRF afvist |
| `404` | Torrent/fil/bruger findes ikke |
| `409` | Fx opsætning allerede gennemført |
| `413` | Body for stor |
| `416` | Ugyldigt Range |
| `429` | Rate limit |
| `5xx` | Serverfejl – se logs |

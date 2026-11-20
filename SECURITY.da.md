# Sikkerhed i NordTorrent

NordTorrent er designet til at køre på et privat netværk. Der er lagt vægt på, at API'et ikke kan bruges uden en gyldig konto, at webpanelet ikke kan CSRF'es, og at Android-tokenet ikke ligger i klartekst på TV'et. Det er stadig dit ansvar at beskytte serveren, netværket og medierne.

## Trusselsmodel

I scope:

- Uautoriseret adgang til webpanel/API fra netværket.
- Stjålne eller lækkede sessionstokens.
- CSRF og clickjacking mod webpanelet.
- Password brute force.
- Utilsigtet eksponering af filer eller tokens i logs.
- En `viewer`-konto, der forsøger at ændre torrents.

Uden for scope:

- En kompromitteret Proxmox-host eller Docker-daemon.
- En ondsindet administrator (admin må tilføje torrents og vælge filer).
- BitTorrent-trafikkens fortrolighed over for peers/ISP.
- Skadelige mediecodecs i browseren/TV'et.
- Fysisk adgang til serveren.

## Kontroller i koden

### Adgangskoder

- Argon2id via `argon2`-crate med standardparametre og tilfældig salt.
- Minimum 12 tegn, maksimum 256 tegn.
- Adgangskoder logges eller returneres aldrig.
- Password-hashing kører på en blocking worker, så API'et ikke blokeres.

### Sessioner

- HMAC-SHA256-signeret token med `iss`, `sub`, `role`, `ver`, `iat`, `exp` og `jti`.
- 32-byte tilfældig hemmelighed gemmes i `data/session-secret`.
- Webpanelet får en `HttpOnly`, `SameSite=Strict`-cookie. Sæt `NORDTORRENT_COOKIE_SECURE=true` bag HTTPS.
- Android får et Bearer-token og gemmer det krypteret med AES-GCM i Android Keystore.
- Password-skift øger brugerens `token_version`, hvilket ugyldiggør alle ældre tokens.

### CSRF og Origin

- Alle ændrende API-kald med cookie kræver headeren `X-Requested-With: NordTorrent`. Den kan ikke sættes af et almindeligt HTML-formular-POST.
- `SameSite=Strict` forhindrer cookien i at følge med cross-site requests.
- `Origin`/`Sec-Fetch-Site` valideres for alle ikke-GET-kald under `/api/`.
- API'et accepterer kun JSON til JSON-endpoints; `multipart/form-data` bruges kun til `.torrent`-upload.

### Roller

| Handling | Admin | Viewer |
| --- | --- | --- |
| Se torrents og bibliotek | ✅ | ✅ |
| Streame mediefiler | ✅ | ✅ |
| Tilføje/fjerne/pause torrents | ✅ | ❌ |
| Vælge filer | ✅ | ❌ |
| Oprette/slette brugere | ✅ | ❌ |
| Se serverstier | ✅ | ❌ |

Den første konto er altid admin. Der kan ikke slettes den sidste admin, og en admin kan ikke slette sig selv.

### Rate limiting

- Login forsøges begrænset til 8 fejl pr. 5 minutter pr. IP+brugernavn.
- Ukendte brugere får samme svar og en konstant forsinkelse for at gøre brugernavnsudforskning sværere.
- Bag en reverse proxy skal `NORDTORRENT_TRUST_PROXY=true` sættes, før `X-Forwarded-For` bruges.

### 2FA og login via proxy

TOTP springes kun over, når forespørgslen reelt er lokal. Serveren kigger på alle de adresser, den kan se: socket-peer, `CF-Connecting-IP` (Cloudflare Tunnel), `True-Client-IP`, `X-Real-IP` og `X-Forwarded-For`. Er blot én af dem en offentlig adresse, regnes login som eksternt, og koden kræves.

Det er netop det, der får Cloudflare Tunnel til at virke: `cloudflared` forbinder fra loopback eller en LAN-adresse, så socket'en alene ser lokal ud, men den videresendte offentlige adresse gør forespørgslen ekstern. En LAN-browser, der taler direkte med serveren, springer stadig 2FA over.

Derfor bliver en LAN-klient, som går gennem en tunnel eller reverse proxy, også bedt om en kode. Det er med vilje: proxyen er det offentlige indgangspunkt. Vil du have det lokale gennembrud, så gå direkte på LAN-adressen.


### Input og filer

- JSON-body er begrænset til 2 MB; `.torrent`-upload til 20 MB.
- API'et accepterer kun magnet-links eller 40-tegns info-hashes til URL-baseret tilføjelse. Dermed kan API'et ikke bruges som vilkårlig HTTP-klient/SSRF.
- Filnavne og torrent-navne vises som tekst i UI'et, ikke som HTML.
- Streaming validerer, at filen findes, er valgt og hører til torrenten.
- Streaming understøtter `Range`, men ingen vilkårlig sti.

### Web-sikkerhed

- CSP med `script-src 'self'`, `object-src 'none'`, `frame-ancestors 'none'`.
- `X-Frame-Options: DENY`, `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`.
- Ingen eksterne CDN'er, analytics eller tredjepartsscripts.
- Ingen CORS-headere; Android bruger native HTTP og er ikke afhængig af browser-CORS.

### Container

- Kører som UID/GID 1000, ikke root.
- `read_only: true` på root-filsystemet i compose, med `tmpfs` til `/tmp`.
- `cap_drop: ALL` og `no-new-privileges:true`.
- Healthcheck mod `/healthz`.

## Krav til drift

1. **Brug HTTPS.** Sæt Caddy, Traefik eller Nginx Proxy Manager foran, og sæt:
   ```env
   NORDTORRENT_PUBLIC_URL=https://torrent.example.dk
   NORDTORRENT_COOKIE_SECURE=true
   NORDTORRENT_TRUST_PROXY=true
   ```
2. **Eksponér ikke port 8080 mod internettet.** Brug firewall/VPN. Webporten bør kun være tilgængelig fra LAN/VPN eller via proxyen.
3. **Brug en viewer-konto til TV'et.** Admin-kontoen bør kun bruges til administration.
4. **Fjern bootstrap-adgangskoden** fra `.env`, når administratoren er oprettet.
5. **Tag backup af `data/`.** `users.json` og `session-secret` er nødvendige for at bevare konti og sessioner.
6. **Opdatér regelmæssigt.** `docker compose build --pull` og genstart.
7. **Beskyt downloadmappen.** Andre tjenester på serveren bør ikke kunne læse/skrive frit i den.

## Kendte begrænsninger

- Der er ikke 2FA/passkeys i denne version.
- Logout sletter webcookien, men et allerede udstedt Bearer-token er gyldigt indtil udløb. Skift adgangskode for at ugyldiggøre alle tokens.
- BitTorrent-trafik er ikke krypteret/obfuskeret af NordTorrent. Din IP og torrent-aktivitet kan være synlig for trackers, DHT og peers.
- Der er ingen indbygget malware-scanning. Download kun fra kilder, du stoler på.
- Webafspilleren afhænger af browserens codec-understøttelse; Android TV bruger ExoPlayer og understøtter flere formater.
- `.torrent`-filer kan indeholde web seeds og trackers. Kun administratorer kan uploade dem.
- Admin er en betroet rolle. Giv ikke admin til brugere, du ikke stoler på.

## Rapportering

Dette er et privat hobbyprojekt. Hvis du finder en sikkerhedsfejl, så undlad at åbne en offentlig issue med følsomme detaljer. Kontakt ejeren af installationen direkte.

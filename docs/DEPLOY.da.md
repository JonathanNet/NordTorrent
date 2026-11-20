# Deployment på Proxmox

Denne guide tager udgangspunkt i en Debian/Ubuntu-baseret VM eller LXC-container på Proxmox med Docker og Docker Compose.

## 1. VM eller LXC?

**Anbefalet: en lille VM** (2 vCPU, 2–4 GB RAM, 16–32 GB systemdisk). Det giver færrest overraskelser med Docker, AppArmor og netværk.

En LXC kan også bruges, men skal have:

- `features: nesting=1,keyctl=1`
- Docker installeret inde i containeren
- Eventuelt `lxc.apparmor.profile: unconfined` og `lxc.cgroup2.devices.allow: c 10:200 rwm` til Docker
- En diskplads, der kan rumme downloads

Brug ikke en privileged LXC, medmindre du har en konkret grund.

## 2. Installér Docker

```bash
sudo apt update
sudo apt install -y ca-certificates curl
sudo install -m 0755 -d /etc/apt/keyrings
sudo curl -fsSL https://download.docker.com/linux/debian/gpg -o /etc/apt/keyrings/docker.asc
sudo chmod a+r /etc/apt/keyrings/docker.asc

echo \
  "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/debian \
  $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | \
  sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

sudo apt update
sudo apt install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
sudo systemctl enable --now docker
```

Tilføj din bruger til `docker`-gruppen, hvis du ikke vil bruge `sudo`:

```bash
sudo usermod -aG docker "$USER"
```

Log ud og ind igen.

## 3. Hent projektet og forbered mapper

```bash
sudo mkdir -p /opt/nordtorrent
sudo chown -R "$USER":"$USER" /opt/nordtorrent
cd /opt/nordtorrent
git clone <dit-repo> .
```

Vælg en data-mappe og en downloadmappe. Downloadmappen kan være lokal disk eller et mount fra NAS:

```bash
sudo mkdir -p /opt/nordtorrent/data /mnt/media/downloads
sudo chown -R 1000:1000 /opt/nordtorrent/data /mnt/media/downloads
```

`DATA_DIR` i `.env` peger på host-mappen, som indeholder brugere, session og DHT-state. Den monteres ind som `/data` i containeren.

Brug `id` til at se din UID/GID:

```bash
id
# uid=1000(jonathan) gid=1000(jonathan) ...
```

Hvis du bruger en anden bruger, sæt `PUID`/`PGID` i `.env`.

## 4. Konfigurér

```bash
cp .env.example .env
nano .env
```

Vigtige værdier:

```env
NORDTORRENT_PUBLIC_URL=http://192.168.1.101:8080
NORDTORRENT_COOKIE_SECURE=false
NORDTORRENT_TRUST_PROXY=false
DATA_DIR=/opt/nordtorrent/data
DOWNLOAD_DIR=/mnt/media/downloads
PUID=1000
PGID=1000
WEB_PORT=8080
TZ=Europe/Copenhagen
```

`DATA_DIR` er **host-stien**, som monteres ind i containeren som `/data`. Selve containerens interne sti skal fortsat være `/data`, så `NORDTORRENT_DATA_DIR` skal normalt ikke ændres i `.env`.

Hvis serveren kun skal bruges på LAN, er HTTP + adgangskode acceptabelt i første omgang. Sæt alligevel HTTPS op, hvis du senere vil nå den udefra.

## 5. Start

```bash
sudo mkdir -p /opt/nordtorrent/data /mnt/media/downloads
sudo chown -R 1000:1000 /opt/nordtorrent/data /mnt/media/downloads
docker compose up -d --build
docker compose logs -f nordtorrent
```

Åbn `http://<server-ip>:8080` og opret administratoren.

Har du flere diske, kan du bruge [`docker-compose.dockhand.yml`](../docker-compose.dockhand.yml). Den monterer hver disk som en undermappe af `/downloads`. NordTorrent opdager mountene automatisk og viser dem ved navn i mappe-vælgeren. Sæt eventuelt `NORDTORRENT_DEFAULT_DOWNLOAD_FOLDER` til den mappe, der skal være forvalgt.

Tjek health:

```bash
docker compose ps
curl -fsS http://127.0.0.1:8080/healthz
```

## 6. Porte og firewall

| Port | Protokol | Formål |
| --- | --- | --- |
| `8080` | TCP | Web/API |
| `42000-42009` | TCP | Indgående peers |
| `42000-42009` | UDP | Reserveret |

På Proxmox kan du bruge **Datacenter → Firewall** eller `ufw` i VM'en.

Anbefalet politik:

- `8080` kun fra LAN/VPN eller fra reverse proxy.
- `42000-42009` TCP/UDP fra internettet, hvis du vil have gode peer-forbindelser.
- **Åbn ikke** `8080` mod internettet.

Eksempel med `ufw`:

```bash
sudo ufw allow from 192.168.1.0/24 to any port 8080 proto tcp
sudo ufw allow 42000:42009/tcp
sudo ufw allow 42000:42009/udp
sudo ufw enable
```

## 7. Host-netværk (valgfrit, men bedre til seeding)

Docker bridge med portmapping virker til web og TCP-peers. DHT bruger en UDP-port, som ikke altid rammer det mappede interval. På Linux giver host-netværk den mest korrekte BitTorrent-adfærd.

Tilføj en override-fil `docker-compose.host.yml`:

```yaml
services:
  nordtorrent:
    network_mode: host
```

Kør:

```bash
docker compose -f docker-compose.yml -f docker-compose.host.yml up -d --build
```

Med host-netværk lytter appen direkte på `0.0.0.0:8080` og på torrent-portene. Docker ignorerer `ports`-mappingen med en advarsel. Sørg for, at firewallen stadig beskytter webporten.

## 8. HTTPS med reverse proxy

Brug gerne Caddy, Traefik eller Nginx Proxy Manager. Et minimalt Caddy-eksempel ligger i [`deploy/Caddyfile`](../deploy/Caddyfile).

Sæt derefter:

```env
NORDTORRENT_PUBLIC_URL=https://torrent.example.dk
NORDTORRENT_COOKIE_SECURE=true
NORDTORRENT_TRUST_PROXY=true
```

og genstart:

```bash
docker compose up -d
```

Proxyen skal videresende `Host`, `X-Forwarded-For` og `X-Forwarded-Proto`. Caddy gør det som standard.

Android TV skal bruge `https://torrent.example.dk` som serveradresse, og certifikatet skal være betroet af TV'et. Et rigtigt Let's Encrypt-certifikat er den nemmeste vej.

## 9. Backup

Alt vigtigt ligger i `data/`:

- `users.json` – konti og password-hashes
- `session-secret` – signeringsnøgle til tokens
- `session/` – librqbit-session
- `dht.json` – DHT-state

```bash
sudo tar -czf /backup/nordtorrent-$(date +%F).tar.gz data/
```

Downloadmappen kan være stor. Tag backup af den separat, hvis du ønsker det.

Gendannelse:

```bash
docker compose down
sudo tar -xzf /backup/nordtorrent-2026-11-14.tar.gz
sudo chown -R 1000:1000 data
docker compose up -d
```

## 10. Opdatering

```bash
cd /opt/nordtorrent
git pull
docker compose build --pull
docker compose up -d
```

Tjek logs og health bagefter.

## 11. Android TV

1. Byg eller hent `nordtorrent-tv-debug.apk`.
2. Installér på TV'et:
   ```bash
   adb connect <tv-ip>:5555
   adb install -r app-debug.apk
   ```
3. Åbn appen.
4. Serveradresse: `http://192.168.1.101:8080` eller `https://torrent.example.dk`.
5. Log ind med en `viewer`-konto.

## 12. Fejlfinding

### Containeren starter ikke

```bash
docker compose logs --tail=100 nordtorrent
```

Typiske årsager:

- `data/` eller downloadmappen ejes af root. Ret ejer til `PUID:PGID`.
- Port 8080 eller 42000 er optaget.
- `NORDTORRENT_ADMIN_USER` er sat uden `NORDTORRENT_ADMIN_PASSWORD` (eller omvendt).

### "Permission denied" på downloads

```bash
sudo chown -R 1000:1000 /mnt/media/downloads
```

Hvis du bruger NFS/SMB, skal mounten have de rigtige uid/gid-muligheder. Overvej at mounte på Proxmox-hostens niveau og bind-mounte ind i VM'en.

### Ingen peers

- Kontrollér at TCP-porten er åben og mappet.
- Prøv host-netværk.
- Kontrollér at trackers/DHT ikke er blokeret af firewall.
- Nogle private trackers kræver, at din port er tilgængelig.

### Android TV kan ikke forbinde

- Brug IP-adressen, ikke `localhost`.
- Kontrollér at TV og server er på samme VLAN.
- Ved HTTPS: kontrollér at certifikatet er gyldigt for det brugte hostnavn.
- Ryd appens data, hvis en gammel token hænger.

## 13. Ressourcer

Til en typisk hjemmeserver:

- 2 vCPU
- 2 GB RAM (4 GB ved mange samtidige torrents)
- 16 GB systemdisk
- Plads til downloads efter behov

NordTorrent transkoder ikke, så CPU-forbruget er lavt sammenlignet med en medieserver.

# Deploying on Proxmox

This guide assumes a Debian/Ubuntu based VM or LXC container on Proxmox with Docker and Docker Compose.

## 1. VM or LXC?

**Recommended: a small VM** (2 vCPU, 2-4 GB RAM, 16-32 GB system disk). It gives the fewest surprises with Docker, AppArmor and networking.

An LXC works too, but needs:

- `features: nesting=1,keyctl=1`
- Docker installed inside the container
- Optionally `lxc.apparmor.profile: unconfined` and `lxc.cgroup2.devices.allow: c 10:200 rwm` for Docker
- Disk space for downloads

Do not use a privileged LXC unless you have a concrete reason.

## 2. Install Docker

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

Add your user to the `docker` group if you do not want to use `sudo`:

```bash
sudo usermod -aG docker "$USER"
```

Log out and back in.

## 3. Get the project and prepare folders

```bash
sudo mkdir -p /opt/nordtorrent
sudo chown -R "$USER":"$USER" /opt/nordtorrent
cd /opt/nordtorrent
git clone <your-repo> .
```

Pick a data folder and a download folder. The download folder can be a local disk or a NAS mount:

```bash
sudo mkdir -p /opt/nordtorrent/data /mnt/media/downloads
sudo chown -R 1000:1000 /opt/nordtorrent/data /mnt/media/downloads
```

`DATA_DIR` in `.env` points at the host folder that holds users, sessions and DHT state. It is mounted into the container as `/data`.

Use `id` to see your UID/GID:

```bash
id
# uid=1000(jonathan) gid=1000(jonathan) ...
```

If you use another user, set `PUID`/`PGID` in `.env`.

## 4. Configure

```bash
cp .env.example .env
nano .env
```

Important values:

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

`DATA_DIR` is the **host path** that is mounted into the container as `/data`. The container path must stay `/data`, so `NORDTORRENT_DATA_DIR` should normally not be changed in `.env`.

If the server is only used on the LAN, HTTP with a password is acceptable to begin with. Set up HTTPS anyway if you later want to reach it from the internet.

## 5. Start

```bash
sudo mkdir -p /opt/nordtorrent/data /mnt/media/downloads
sudo chown -R 1000:1000 /opt/nordtorrent/data /mnt/media/downloads
docker compose up -d --build
docker compose logs -f nordtorrent
```

Open `http://<server-ip>:8080` and create the administrator.

With several disks you can use [`docker-compose.dockhand.yml`](../docker-compose.dockhand.yml). It mounts each disk as a subfolder of `/downloads`. NordTorrent discovers the mounts and lists them by name in the folder picker. Optionally set `NORDTORRENT_DEFAULT_DOWNLOAD_FOLDER` to the folder that should be preselected.

Check health:

```bash
docker compose ps
curl -fsS http://127.0.0.1:8080/healthz
```

## 6. Ports and firewall

| Port | Protocol | Purpose |
| --- | --- | --- |
| `8080` | TCP | Web/API |
| `42000-42009` | TCP | Incoming peers |
| `42000-42009` | UDP | Reserved |

On Proxmox you can use **Datacenter -> Firewall** or `ufw` inside the VM.

Recommended policy:

- `8080` only from LAN/VPN or from a reverse proxy.
- `42000-42009` TCP/UDP from the internet if you want good peer connectivity.
- Do **not** open `8080` to the internet.

Example with `ufw`:

```bash
sudo ufw allow from 192.168.1.0/24 to any port 8080 proto tcp
sudo ufw allow 42000:42009/tcp
sudo ufw allow 42000:42009/udp
sudo ufw enable
```

## 7. Host networking (optional, but better for seeding)

Docker bridge with port mapping works for the web UI and TCP peers. DHT uses a UDP port that does not always land inside the mapped range. On Linux, host networking gives the most correct BitTorrent behaviour.

Add an override file `docker-compose.host.yml`:

```yaml
services:
  nordtorrent:
    network_mode: host
```

Run:

```bash
docker compose -f docker-compose.yml -f docker-compose.host.yml up -d --build
```

With host networking the app listens directly on `0.0.0.0:8080` and on the torrent ports. Docker ignores the `ports` mapping with a warning. Make sure the firewall still protects the web port.

## 8. HTTPS with a reverse proxy

Use Caddy, Traefik or Nginx Proxy Manager. A minimal Caddy example is in [`deploy/Caddyfile`](../deploy/Caddyfile).

Then set:

```env
NORDTORRENT_PUBLIC_URL=https://torrent.example.com
NORDTORRENT_COOKIE_SECURE=true
NORDTORRENT_TRUST_PROXY=true
```

and restart:

```bash
docker compose up -d
```

The proxy must forward `Host`, `X-Forwarded-For` and `X-Forwarded-Proto`. Caddy does this by default.

Android TV must use `https://torrent.example.com` as the server address, and the TV must trust the certificate. A real Let's Encrypt certificate is the easiest path.

## 9. Backup

Everything important lives in `data/`:

- `users.json` - accounts and password hashes
- `session-secret` - token signing key
- `session/` - librqbit session
- `dht.json` - DHT state

```bash
sudo tar -czf /backup/nordtorrent-$(date +%F).tar.gz data/
```

The download folder can be large. Back it up separately if you want to.

Restore:

```bash
docker compose down
sudo tar -xzf /backup/nordtorrent-2026-11-14.tar.gz
sudo chown -R 1000:1000 data
docker compose up -d
```

## 10. Updating

```bash
cd /opt/nordtorrent
git pull
docker compose build --pull
docker compose up -d
```

Check logs and health afterwards.

## 11. Android TV

1. Build or download `nordtorrent-tv-debug.apk`.
2. Install it on the TV:
   ```bash
   adb connect <tv-ip>:5555
   adb install -r app-debug.apk
   ```
3. Open the app.
4. Server address: `http://192.168.1.101:8080` or `https://torrent.example.com`.
5. Sign in with a `viewer` account.

## 12. Troubleshooting

### The container does not start

```bash
docker compose logs --tail=100 nordtorrent
```

Typical causes:

- `data/` or the download folder is owned by root. Fix ownership to `PUID:PGID`.
- Port 8080 or 42000 is already in use.
- `NORDTORRENT_ADMIN_USER` is set without `NORDTORRENT_ADMIN_PASSWORD` (or the other way around).

### "Permission denied" on downloads

```bash
sudo chown -R 1000:1000 /mnt/media/downloads
```

If you use NFS/SMB, the mount needs the right uid/gid options. Consider mounting on the Proxmox host and bind-mounting into the VM.

### No peers

- Check that the TCP port is open and mapped.
- Try host networking.
- Check that trackers/DHT are not blocked by the firewall.
- Some private trackers require your port to be reachable.

### Android TV cannot connect

- Use the IP address, not `localhost`.
- Check that the TV and the server are on the same VLAN.
- With HTTPS: check that the certificate is valid for the hostname used.
- Clear the app data if an old token is stuck.

## 13. Resources

For a typical home server:

- 2 vCPU
- 2 GB RAM (4 GB with many simultaneous torrents)
- 16 GB system disk
- Disk space for downloads as needed

NordTorrent does not transcode, so CPU usage is low compared with a media server.

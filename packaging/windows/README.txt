NordTorrent for Windows
=======================

This folder contains a prebuilt Windows build of NordTorrent.

Quick start
-----------

1. Double-click run-nordtorrent.cmd.
2. Your browser opens http://127.0.0.1:8080
3. Create the administrator account (at least 12 characters).
4. Add a magnet link or a .torrent file.

The launcher keeps everything inside this folder:

  data\       users, sessions, settings, statistics
  downloads\  the media library

Copy the whole folder to another disk and the install moves with it.
Move the folder while the server is stopped.

Using another download folder
-----------------------------

Edit run-nordtorrent.cmd and change NORDTORRENT_DOWNLOAD_DIR, for example:

  set "NORDTORRENT_DOWNLOAD_DIR=D:\Media\Downloads"

Use an absolute path. Subfolders of that folder appear as separate download
folders in the web panel.

Reaching it from other devices
------------------------------

By default the server listens on 0.0.0.0:8080, so a phone or TV on the same
network can use http://<this-pc-ip>:8080.

Set NORDTORRENT_PUBLIC_URL in run-nordtorrent.cmd to the exact address you
open in the browser (for example http://192.168.1.50:8080). This is used for
the login origin check. If the address does not match, signing in fails with
"Origin is not allowed".

For access from the internet, put HTTPS in front of it (Caddy, Traefik, a
Cloudflare Tunnel, or Tailscale). Do not expose port 8080 directly.

Firewall
--------

Windows asks once whether to allow nordtorrent.exe. Allow it on private
networks so the TV and phone can connect. BitTorrent peer traffic uses TCP
42000-42009; forward those ports in your router if you want good connectivity.

Optional: video thumbnails
--------------------------

The TV library generates preview images with ffmpeg. If ffmpeg is not on PATH,
set NORDTORRENT_FFMPEG in run-nordtorrent.cmd to the full path, for example:

  set "NORDTORRENT_FFMPEG=C:\ffmpeg\bin\ffmpeg.exe"

Without ffmpeg everything still works; the library shows icons instead of
previews.

Stopping and updating
---------------------

Stop with Ctrl+C or by closing the console window. To update, replace
nordtorrent.exe with a newer build and start it again. The data folder is
untouched.

More documentation
------------------

README.md in this folder, and the API reference in docs/API.md of the source
archive.

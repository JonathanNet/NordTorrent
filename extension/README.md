# NordTorrent Download Helper

Chrome extension (Manifest V3) that sends `.torrent` downloads straight to your NordTorrent server instead of saving them locally.

## Installation

1. Open `chrome://extensions`.
2. Enable **Developer mode**.
3. Choose **Load unpacked** and select this `extension/` folder.
4. Click the extension icon.
5. Enter:
   - Server: `http://192.168.1.10:8080`
   - Username and password for an **admin** account
6. Save.

The password is never stored. Only the login token is kept in `chrome.storage.local`.

## Language

The extension is in English by default. Open the settings (the gear icon) and pick **Dansk** under **Language** to switch to Danish; the choice is stored per browser and applied immediately.

## Popup on/off

**Open popup on .torrent downloads** can be toggled in three places:

- the main view
- the settings view
- the login view

The setting is saved immediately. When disabled, `.torrent` downloads continue as a completely normal Chrome download without a popup.

## Labels

When adding a `.torrent` file or a magnet link you can pick one of the server's labels in the **Label** dropdown. Labels are created and managed in the web panel under **Labels**.

## Magnet links

When you press **Copy magnet link** on a page, the extension opens its popup window with the magnet link already filled in. This covers `navigator.clipboard.writeText`, `document.execCommand("copy")`, copying selected text and buttons with `data-clipboard-text`/magnet attributes.

You can also open the extension and paste a `magnet:?xt=...` link or a 40-character info hash into the magnet field. Pick a download folder and press **Add magnet**.

The download folder list has a search field so you can quickly filter folders in the current directory.

## How it works

When Chrome starts a `.torrent` download:

- a small NordTorrent window opens
- Chrome's own download is cancelled immediately so the **Save As** dialog does not linger
- the extension fetches the `.torrent` file with the browser's cookies
- you pick a download folder on the server
- the file is uploaded to `/api/v1/torrents/upload`
- the local Chrome download is cancelled and the file removed

If Chrome cannot fetch the file automatically (for example a protected or short-lived download URL), you can choose the `.torrent` file manually in the window.

## Permissions

- `downloads`: detect, cancel and remove the local `.torrent` download.
- `storage`: store the server, username and login token.
- `<all_urls>`: fetch the `.torrent` file from the page you download from, and call the NordTorrent server.

The extension has no access to server files beyond the API calls the signed-in account is allowed to make.

## Limitations

- `.torrent` downloads only; magnet links are handled in the web panel.
- Some private trackers require a specific `Referer` or a one-time token that Chrome does not expose to the extension. Use the file picker in that case.
- The server must be updated with the `extension-api` feature.
- Restart Chrome after installing if the download popup does not appear.

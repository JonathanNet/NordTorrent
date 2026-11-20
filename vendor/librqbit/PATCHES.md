# Local librqbit patch

Source: crates.io `librqbit` 8.1.1 (Apache-2.0).

## Private tracker handling

`src/session.rs`, in `Session::make_peer_rx`:

- Use the first tracker embedded in a private torrent.  The other entries are
  normally mirrors of the same tracker; announcing to all of them makes the
  tracker see duplicate peers and ignore the completed event.
- Never add the session-wide/public fallback trackers to a private info hash.
- Keep DHT disabled for private torrents.

## HTTP tracker user agent

`src/session.rs`, where the shared `reqwest::Client` is built:

- Set `User-Agent: NordTorrent/0.1.0`.
- NordicBytes and similar private trackers reject the default reqwest/Nothing
  user agent with `Invalid user agent`.

Both changes are intentionally small so they can be dropped when upstream
implements private tracker tiers and a configurable user agent.

## Torrent-name subfolder

`src/session.rs`, output-folder selection:

When an explicit `output_folder` is supplied, multi-file torrents now get a
torrent-name subfolder below it, matching rTorrent/ruTorrent. Single-file
torrents keep writing directly to the chosen folder.

## Tracker completed announce

See `../librqbit-tracker-comms/PATCHES.md`. HTTP trackers now receive the
one-shot `event=completed` announce, which private tracker sites use for
ratio/seedtime accounting.

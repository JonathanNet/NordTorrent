# Local librqbit-tracker-comms patch

Source: crates.io `librqbit-tracker-comms` 3.0.0 (Apache-2.0).

## HTTP `event=completed`

`src/tracker_comms.rs`, `task_single_tracker_monitor_http`:

Upstream only sent `event=started` once and then never sent
`event=completed`. Private trackers use that event to register the download as
finished and to start seedtime accounting.

The patch sends exactly one `event=completed` announce after the torrent
reaches 100%, then continues with normal announces.

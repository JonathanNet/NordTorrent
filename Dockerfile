# syntax=docker/dockerfile:1.7
FROM rust:1.99-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        clang \
        cmake \
        libclang-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY server/Cargo.toml server/Cargo.lock ./server/
COPY server/src ./server/src
COPY vendor ./vendor
COPY web ./web

RUN cargo build --release --locked --manifest-path server/Cargo.toml

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        ffmpeg \
        tini \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 1000 --shell /usr/sbin/nologin nordtorrent \
    && mkdir -p /data /downloads \
    && chown -R nordtorrent:nordtorrent /data /downloads

COPY --from=builder /src/server/target/release/nordtorrent /usr/local/bin/nordtorrent

ENV NORDTORRENT_LISTEN=0.0.0.0:8080 \
    NORDTORRENT_DATA_DIR=/data \
    NORDTORRENT_DOWNLOAD_DIR=/downloads \
    RUST_LOG=nordtorrent=info,tower_http=warn,librqbit=warn

EXPOSE 8080
# Torrent peer ports 42000-42009 are published by docker-compose.yml (TCP and UDP).

# Volumes are declared by docker-compose.yml / docker-compose.dockhand.yml.
# Keeping them out of the image avoids an anonymous /downloads volume that would
# otherwise show up as an extra, non-persistent download folder.

USER 1000:1000
STOPSIGNAL SIGTERM

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8080/healthz || exit 1

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/nordtorrent"]

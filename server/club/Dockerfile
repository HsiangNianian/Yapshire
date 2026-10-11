FROM rust:1.95-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY src ./src
COPY assets/maps ./assets/maps
COPY assets/packs ./assets/packs
RUN cargo build --release --locked -p yapshire-server

# Reuse the released non-root runtime, replacing its server with this workspace.
FROM ghcr.io/hsiangnianian/yapshire-server@sha256:68ba85278ff4d6fd0d90a6593a357737bd5c6e4ab895ef448c9aa7b1a66d0da5
COPY --from=build /src/target/release/yapshire-server /usr/local/bin/yapshire-server
COPY --chown=10001:10001 server/club/server.json /data/server.json
RUN yapshire-server --check

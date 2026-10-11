FROM rust:1.95-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY src ./src
COPY assets/maps ./assets/maps
COPY assets/packs ./assets/packs
RUN cargo build --release --locked -p yapshire-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir /data && chown 10001:10001 /data
COPY --from=build /src/target/release/yapshire-server /usr/local/bin/yapshire-server
COPY LICENSE.md /usr/share/doc/yapshire-server/LICENSE.md
USER 10001:10001
WORKDIR /data
EXPOSE 4761
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl --fail --silent http://127.0.0.1:4761/health || exit 1
ENTRYPOINT ["yapshire-server"]

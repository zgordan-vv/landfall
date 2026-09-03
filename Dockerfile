FROM rust:1.98.0-bookworm@sha256:82150a52ec202c1b14d7817e14516c392bb7f5cfebd88f1ed531cb37ebd39922 AS builder

WORKDIR /workspace

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --locked --release --bin landfall-server

FROM debian:bookworm-slim@sha256:88200866dfff7ea7f5cbcb6ec7c8a701889efe6fe859fe64d6990e4b07ea4171 AS runtime

LABEL org.opencontainers.image.title="Landfall Server" \
      org.opencontainers.image.description="Self-hosted Solana transaction observability server" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.source="https://github.com/zgordan-vv/landfall"

COPY --from=builder /workspace/target/release/landfall-server /usr/local/bin/landfall-server
COPY LICENSE /licenses/LICENSE

USER 65532:65532

ENTRYPOINT ["/usr/local/bin/landfall-server"]

FROM node:24-bookworm-slim AS dashboard-builder

WORKDIR /workspace

RUN corepack enable

COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY tsconfig.json tsconfig.base.json ./
COPY apps/dashboard/package.json apps/dashboard/package.json
COPY packages/api-client/package.json packages/api-client/package.json
COPY packages/protocol-ts/package.json packages/protocol-ts/package.json
RUN pnpm install --frozen-lockfile --config.runtime-on-fail=ignore

COPY apps/dashboard apps/dashboard
COPY packages/api-client packages/api-client
COPY packages/protocol-ts packages/protocol-ts
RUN pnpm --config.runtime-on-fail=ignore --filter @landfall/api-client... build \
    && pnpm --config.runtime-on-fail=ignore --filter @landfall/dashboard build

FROM rust:1.98.0-bookworm@sha256:82150a52ec202c1b14d7817e14516c392bb7f5cfebd88f1ed531cb37ebd39922 AS builder

WORKDIR /workspace

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY migrations ./migrations

RUN cargo build --locked --release --bin landfall-server

FROM debian:bookworm-slim@sha256:88200866dfff7ea7f5cbcb6ec7c8a701889efe6fe859fe64d6990e4b07ea4171 AS runtime

LABEL org.opencontainers.image.title="Landfall Server" \
      org.opencontainers.image.description="Self-hosted Solana transaction observability server" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.source="https://github.com/zgordan-vv/landfall"

COPY --from=builder /workspace/target/release/landfall-server /usr/local/bin/landfall-server
COPY --from=dashboard-builder /workspace/apps/dashboard/dist /opt/landfall/dashboard
COPY LICENSE /licenses/LICENSE

USER 65532:65532

STOPSIGNAL SIGTERM

# Debian slim has no curl/wget; this verifies that the PID 1 process is alive.
# HTTP readiness is probed by the orchestrator against /health/ready.
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD ["/usr/bin/kill", "-0", "1"]

ENTRYPOINT ["/usr/local/bin/landfall-server"]

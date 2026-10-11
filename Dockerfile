# Local image built from source on any host (feature 008, research R6). Releases use
# deploy/docker/Dockerfile.release with the pipeline-built binaries instead (Principle V).
# Pinned (docs/rust-guidelines.md §2.2); keep in step with rust-version in .circleci/config.yml.
ARG RUST_VERSION=1.99

FROM rust:${RUST_VERSION}-trixie AS build
RUN apt-get update \
    && apt-get install -y --no-install-recommends musl-tools \
    && rm -rf /var/lib/apt/lists/*
ARG TARGETARCH
RUN case "${TARGETARCH}" in \
        arm64) echo aarch64-unknown-linux-musl ;; \
        *) echo x86_64-unknown-linux-musl ;; \
    esac > /rust-target \
    && rustup target add "$(cat /rust-target)"
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked --target "$(cat /rust-target)" \
    && cp "target/$(cat /rust-target)/release/live-qso-map" /live-qso-map \
    && apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && mkdir -p /out/etc/live-qso-map /out/var/lib/live-qso-map \
    && chown 10001:10001 /out/var/lib/live-qso-map \
    && chmod 0750 /out/var/lib/live-qso-map

# Same runtime as the release image: Debian 13 slim, unprivileged user, data volume.
FROM debian:trixie-slim
LABEL org.opencontainers.image.title="live-qso-map" \
      org.opencontainers.image.source="https://github.com/sardylan/live-qso-map" \
      org.opencontainers.image.licenses="GPL-3.0-only"
COPY --from=build /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=build /out/etc/live-qso-map /etc/live-qso-map
COPY --from=build --chown=10001:10001 /out/var/lib/live-qso-map /var/lib/live-qso-map
COPY LICENSE.md /usr/share/doc/live-qso-map/LICENSE.md
COPY --chmod=755 deploy/docker/entrypoint.sh /usr/local/bin/live-qso-map-entrypoint
COPY --from=build /live-qso-map /usr/bin/live-qso-map
USER 10001:10001
VOLUME /var/lib/live-qso-map
EXPOSE 12060/udp 8080/tcp
ENTRYPOINT ["/usr/local/bin/live-qso-map-entrypoint"]

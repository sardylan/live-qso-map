# Multi-stage build: a static musl binary on a distroless non-root image (research R18).
ARG RUST_VERSION=1.98

FROM rust:${RUST_VERSION} AS build
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
    && mkdir /data

# distroless/static ships CA certificates, needed for the provider and cty.dat downloads.
FROM gcr.io/distroless/static-debian12:nonroot
COPY --from=build /live-qso-map /live-qso-map
COPY --from=build --chown=65532:65532 /data /data
ENV LQM_CTY_PATH=/data/cty.dat
VOLUME /data
EXPOSE 12060/udp 8080/tcp
USER nonroot
ENTRYPOINT ["/live-qso-map"]

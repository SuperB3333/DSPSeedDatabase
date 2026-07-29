ARG IMAGE_VERSION=dev
ARG VCS_REF=unknown

FROM rust:1.97.0@sha256:b92b8c8574f8f3b207fcb0912fb3e2de4041580b5934d90312d53938c9a038a9 AS builder
WORKDIR /app
COPY ./inserter/ .
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --locked --target x86_64-unknown-linux-musl --bin dsp_seed_finder

FROM scratch
ARG IMAGE_VERSION
ARG VCS_REF
LABEL org.opencontainers.image.title="DSP Seed Finder" \
      org.opencontainers.image.version="${IMAGE_VERSION}" \
      org.opencontainers.image.revision="${VCS_REF}" \
      org.opencontainers.image.source="https://github.com/SuperB3333/DSPSeedDatabase"
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/dsp_seed_finder /app/app
CMD ["/app/app"]

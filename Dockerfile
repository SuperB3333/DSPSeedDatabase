FROM rust:latest AS builder
WORKDIR /app
COPY ./inserter/ .
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --target x86_64-unknown-linux-musl --bin dsp_seed_finder

FROM scratch
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/dsp_seed_finder /app/app
CMD ["/app/app"]

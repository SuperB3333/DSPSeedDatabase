FROM rust:latest AS builder
WORKDIR /app
COPY ./inserter/* .
RUN cargo build --release

FROM scratch
COPY --from=builder /app/target/release/dsp_seed_finder /app/app
CMD ["/app/app"]
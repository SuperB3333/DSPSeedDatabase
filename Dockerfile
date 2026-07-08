FROM rust:latest AS builder
WORKDIR /app
<<<<<<< HEAD
COPY ./inserter/* .
RUN cargo build --release
=======
COPY . .
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --target x86_64-unknown-linux-musl
>>>>>>> c33cd78 (fix: replaced dynamically linked binary with static one)

FROM scratch
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/dsp_seed_finder /app/app
CMD ["/app/app"]

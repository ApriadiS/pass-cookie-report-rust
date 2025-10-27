# Dockerfile for x86_64 architecture
FROM rust:1.82-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconf build-base perl make

ENV OPENSSL_STATIC=1
ENV OPENSSL_DIR=/usr

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release --target=x86_64-unknown-linux-musl

COPY src ./src
RUN touch src/main.rs
RUN cargo build --release --target=x86_64-unknown-linux-musl

FROM alpine:latest

RUN apk add --no-cache libssl3 ca-certificates

WORKDIR /app
COPY --from=builder /usr/src/app/target/x86_64-unknown-linux-musl/release/pass-cookie-report-rust .

EXPOSE 3000
CMD ["./pass-cookie-report-rust"]


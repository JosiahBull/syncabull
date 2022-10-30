FROM lukemathwalker/cargo-chef:latest-rust-alpine AS chef
WORKDIR /app

RUN rustup toolchain install nightly-x86_64-unknown-linux-musl
RUN rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-musl

RUN apk upgrade --update-cache --available && apk add openssl openssl-dev pkgconfig

FROM chef as planner
COPY . .
RUN cargo +nightly chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo +nightly chef cook --profile production --target x86_64-unknown-linux-musl --recipe-path recipe.json

# Build application
COPY . .
RUN cargo +nightly build -Z build-std=std,panic_abort --target x86_64-unknown-linux-musl --profile production

# We do not need the Rust toolchain to run the binary!
FROM alpine AS runtime
WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/production/syncabull_api /app/

ENV HOST 0.0.0.0
ENV PORT 3000

EXPOSE 3000

CMD [ "/app/syncabull_api" ]

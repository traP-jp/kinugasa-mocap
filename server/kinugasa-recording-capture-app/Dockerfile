FROM rust:1.95-bookworm AS builder

WORKDIR /src
COPY . .

RUN cargo build --release \
    -p kinugasa-recording-capture-app \
    --bin kinugasa-recording-capture-app

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/release/kinugasa-recording-capture-app /usr/local/bin/kinugasa-recording-capture-app

ENTRYPOINT ["/usr/local/bin/kinugasa-recording-capture-app"]

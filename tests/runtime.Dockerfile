FROM rust:1.95-bookworm AS builder

ARG LIBRIST_REV=91a88de284542ec0414e32e5b884ba33b3fd3b91

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        clang \
        git \
        libclang-dev \
        meson \
        ninja-build \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN git clone https://code.videolan.org/rist/librist.git /opt/librist-src \
    && git -C /opt/librist-src checkout "${LIBRIST_REV}"

WORKDIR /src
COPY . .

ENV LIBRIST_SRC=/opt/librist-src
RUN cargo build \
    -p kinugasa-test-tools --bin protocol_e2e \
    -p kinugasa-gst --bin rist_receive

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        ffmpeg \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/debug/protocol_e2e /usr/local/bin/protocol_e2e
COPY --from=builder /src/target/debug/rist_receive /usr/local/bin/rist_receive

WORKDIR /test

ARG RUST_VERSION=1.81.0
ARG APP_NAME=hydrus-ai-tagger
FROM rust:${RUST_VERSION} AS build
ARG APP_NAME
WORKDIR /app
RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    <<EOF
set -e
cargo build --release
cp ./target/release/$APP_NAME /bin/rustapp
EOF

FROM nvcr.io/nvidia/cuda:12.6.1-base-ubuntu24.04 AS final
RUN apt-get update && apt-get install -y python3 python3-pip && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*
RUN pip3 install onnxruntime-gpu --break-system-packages

COPY --from=build /bin/rustapp /bin/rustapp
RUN echo '#!/bin/bash\n\
    export ORT_DYLIB_PATH=$(find /usr/local/lib -name "libonnxruntime.so*" | head -n 1)\n\
    export ORT_STRATEGY=system\n\
    exec "$@"' > /entrypoint.sh && chmod +x /entrypoint.sh

ENV ORT_STRATEGY=system
ENTRYPOINT ["/entrypoint.sh"]
CMD ["rustapp", "daemon"]

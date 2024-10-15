ARG RUST_VERSION=1.81.0
ARG APP_NAME=hydrus-ai-tagger
FROM rust:${RUST_VERSION} AS build
WORKDIR /app
RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    <<EOF
set -e
cargo build --release --features load-dynamic
cp ./target/release/$APP_NAME /bin/$APP_NAME
EOF


FROM nvcr.io/nvidia/tensorrt:24.09-py3 AS tensorrt
ARG APP_NAME
RUN apt-get update && apt-get install -y python3 python3-pip python3-venv && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*
RUN python3 -m venv /venv
RUN /venv/bin/python -m pip install onnxruntime-gpu

COPY --from=build /bin/$APP_NAME /bin/$APP_NAME
COPY --chmod=0755 ./entrypoint.sh /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
CMD [${APP_NAME}, "daemon"]

FROM nvcr.io/nvidia/cuda:12.6.2-cudnn-runtime-ubuntu22.04 AS cuda
ARG APP_NAME
RUN apt-get update && apt-get install -y python3 python3-pip python3-venv && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*
RUN python3 -m venv /venv
RUN /venv/bin/python -m pip install onnxruntime-gpu

COPY --from=build /bin/$APP_NAME /bin/$APP_NAME
COPY --chmod=0755 ./entrypoint.sh /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
CMD [${APP_NAME}, "daemon"]

FROM ubuntu:22.04 AS cpu
ARG APP_NAME
RUN apt-get update && apt-get install -y python3 python3-pip python3-venv && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*
RUN python3 -m venv /venv
RUN /venv/bin/python -m pip install onnxruntime

COPY --from=build /bin/$APP_NAME /bin/$APP_NAME
COPY --chmod=0755 ./entrypoint.sh /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
CMD [${APP_NAME}, "daemon"]
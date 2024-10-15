#!/bin/bash
export ORT_DYLIB_PATH=$(find /venv -name "libonnxruntime.so*" | head -n 1)
export ORT_STRATEGY=system
exec "$@"
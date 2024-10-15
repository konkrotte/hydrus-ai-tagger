#!/bin/bash
export ORT_DYLIB_PATH=$(find /usr/local/lib -name "libonnxruntime.so*" | head -n 1)
export ORT_STRATEGY=system
exec "$@"
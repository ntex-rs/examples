#!/bin/sh
# Regenerates src/echo.rs from echo.proto.
#
# Needs the `ntex-grpc` code generator and `protoc`:
#
#   cargo install ntex-grpc-codegen
#
# Run it from this directory whenever echo.proto changes, and commit
# the result. Nothing is generated at build time.
set -e
ntex-grpc echo.proto echo.rs --out-dir ./src --include-dir ./ \
    --rustfmt-path ../../rustfmt.toml

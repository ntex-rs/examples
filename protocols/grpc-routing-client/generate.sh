#!/bin/sh
# Regenerates src/shop.rs from shop.proto.
#
# Needs the `ntex-grpc` code generator and `protoc`:
#
#   cargo install ntex-grpc-codegen
#
# Run it from this directory whenever shop.proto changes, and commit the
# result. Nothing is generated at build time.
set -e
ntex-grpc shop.proto shop.rs --out-dir ./src --include-dir ./ \
    --rustfmt-path ../../rustfmt.toml

#!/bin/sh
# Regenerates the Rust code for inventory.proto.
#
# Needs the `ntex-grpc` code generator and `protoc`:
#
#   cargo install ntex-grpc-codegen
#
# Run it from this directory whenever inventory.proto changes, and commit
# the result. Nothing is generated at build time.
set -e

# src/inventory.rs: `request_id` and `sku` fields use the types in
# src/types.rs. A path without a leading dot matches by suffix, so
# `request_id` covers the field in both messages.
ntex-grpc inventory.proto inventory.rs --out-dir ./src --include-dir ./ \
    --rustfmt-path ../../rustfmt.toml \
    --map request_id=crate::types::RequestId \
    --map sku=crate::types::Sku

# src/plain/inventory.rs: the same messages with the default types, used by
# the tests as a client that knows nothing about the custom types. The output
# file is named after the proto package, so it needs its own directory.
mkdir -p ./src/plain
ntex-grpc inventory.proto inventory.rs --out-dir ./src/plain --include-dir ./ \
    --rustfmt-path ../../rustfmt.toml

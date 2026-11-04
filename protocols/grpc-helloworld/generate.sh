#!/bin/sh
# Regenerates src/helloworld.rs from helloworld.proto.
#
# Needs the `ntex-grpc` code generator and `protoc`:
#
#   cargo install ntex-grpc-codegen
#
# Run it from this directory whenever helloworld.proto changes, and commit
# the result. Nothing is generated at build time.
set -e
ntex-grpc helloworld.proto helloworld.rs --out-dir ./src --include-dir ./ \
    --rustfmt-path ../../rustfmt.toml

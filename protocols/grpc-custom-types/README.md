# gRPC: custom field types

This example shows how to:

- replace the type of generated message fields with your own Rust types,
  using the `--map` option of `ntex-grpc-codegen`;
- implement `NativeType` to define a type's wire format, its default value and
  which values it accepts;
- reject invalid input while the message is decoded, before any service code
  runs.

| Binary | Source | What it does |
| --- | --- | --- |
| `grpc-custom-types-server` | [`src/main.rs`](src/main.rs) | Serves `inventory.Inventory` on `127.0.0.1:50052` |
| `grpc-custom-types-client` | [`src/client.rs`](src/client.rs) | Reserves stock and prints the reply |

The custom types are in [`src/types.rs`](src/types.rs) and the service in
[`src/lib.rs`](src/lib.rs). [`src/inventory.rs`](src/inventory.rs) and
[`src/plain/inventory.rs`](src/plain/inventory.rs) are generated; don't edit
them.

## Before you start

- Run every command below from this directory, `protocols/grpc-custom-types`.
- You only need `protoc` and the code generator to regenerate the code after
  editing [`inventory.proto`](inventory.proto):

  ```sh
  cargo install ntex-grpc-codegen
  ./generate.sh
  ```

## Run it

Start the server in one terminal:

```sh
cargo run --bin grpc-custom-types-server
```

The server starts with 5 `BOOK-42` and 100 `PEN-7`. Reserve some from another
terminal:

```sh
cargo run --bin grpc-custom-types-client -- BOOK-42 2
cargo run --bin grpc-custom-types-client -- BOOK-42 4
```

The first call prints `reserved 2 x BOOK-42, 3 left`, the second
`not enough BOOK-42, 3 left`. An invalid SKU such as `"book 42"` never leaves
the client, because a `Sku` can't hold it.

Other clients only see the proto types, so they can send anything. With
[grpcurl](https://github.com/fullstorydev/grpcurl), the server rejects the
same SKU with `InvalidArgument`:

```sh
grpcurl -plaintext -proto inventory.proto -d '{"sku": "book 42", "quantity": 1}' \
    127.0.0.1:50052 inventory.Inventory/Reserve
```

## What to look for in the code

- **Mapping:** `generate.sh` passes `--map request_id=crate::types::RequestId`
  and `--map sku=crate::types::Sku`. A path without a leading dot matches by
  suffix, so each option covers the field in both messages. The generated
  structs then have `RequestId` and `Sku` fields.
- **`RequestId`:** implements the wire format itself. It is a `bytes` field
  (`WireType::LengthDelimited`) holding the 16 UUID bytes, and the nil UUID is
  its default, so it is left out of the message like an empty field.
- **`Sku`:** delegates the wire format to `ByteString` and adds the SKU rules
  in `merge`. A decode error makes the server answer `INVALID_ARGUMENT`
  without calling `reserve`.
- **Wire compatibility:** `src/plain/inventory.rs` is generated from the same
  proto without `--map`. The tests use it as a client that knows nothing about
  the custom types.

## Learn more

From the guide chapter [Building gRPC Services and Clients](https://docs.ntex.rs/11-grpc.html):

- [Using your own field types](https://docs.ntex.rs/11-grpc.html#using-your-own-field-types) with `--map`
- [Writing a custom field type](https://docs.ntex.rs/11-grpc.html#writing-a-custom-field-type)

## Tests

```sh
cargo test -p grpc-custom-types
```

The tests check that the custom types keep the proto wire format, that invalid values fail to decode and that the server answers them with `INVALID_ARGUMENT`, and they make reservations through the server.

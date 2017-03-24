# dkregistry-rs

`dkregistry` is a Rust library for asynchronous interaction with container registries
conformant to the [Docker Registry HTTP API V2][registry-v2] specification.

[registry-v2]: https://docs.docker.com/registry/spec/api/

## Testing

This library relies on [mockito][mockito-gh] for tests and mocking, which is not multi-thread aware.

As such, tests should be run serially via:
```
cargo test -- --test-threads=1
```

[mockito-gh]: https://github.com/lipanski/mockito

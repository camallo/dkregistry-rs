# dkregistry


[![Build Status](https://travis-ci.org/camallo/dkregistry-rs.svg?branch=master)](https://travis-ci.org/camallo/caps-rs)
[![LoC](https://tokei.rs/b1/github/camallo/dkregistry-rs?category=code)](https://github.com/camallo/caps-rs)

A pure-Rust asynchronous library for Docker Registry API.

`dkregistry` provides support for asynchronous interaction with container registries
conformant to the [Docker Registry HTTP API V2][registry-v2] specification.

[registry-v2]: https://docs.docker.com/registry/spec/api/

## Testing

### Integration tests

This library relies on the [mockito][mockito-gh] framework for mocking. At this time, it is not multi-thread aware.

As such, tests should be run serially via:
```
cargo test -- --test-threads=1
```

[mockito-gh]: https://github.com/lipanski/mockito

### Interoperability tests

This library includes additional interoperability tests against some of the most common registries.

Those tests are not run by default as they required network access and registry credentials.

They are gated behind a dedicated "test-net" feature and can be run as:
```
cargo test --features test-net
```

Credentials for those registries must be provided via environmental flags.

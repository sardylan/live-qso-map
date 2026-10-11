# Rust Implementation Guidelines

These rules are binding for all Rust code in this repository. Constitution Principle II makes
them so (`.specify/memory/constitution.md`). Plans check them in their Constitution Check, and
code review enforces them.

The constitution takes precedence. This guide adds detail and can be changed without amending
it: edit this file, then note the change in the [changelog](#changelog) at the bottom.

## 1. Errors and `Result`

### 1.1 One crate error type

The crate MUST define exactly one error type, `enum Error`, in `src/errors.rs`. It MUST:

- implement `std::fmt::Display`;
- implement `std::error::Error`, overriding `source()` whenever a variant wraps an
  underlying error;
- derive `Debug`. A handwritten `Debug` is allowed when a variant holds secrets, which must be
  redacted.

### 1.2 Crate `Result` alias

`src/errors.rs` MUST declare:

```rust
pub type Result<T> = std::result::Result<T, Error>;
```

Every fallible function in the crate MUST return this `Result<T>`. In particular:

- MUST NOT spell out `std::result::Result<T, Error>` where the alias applies;
- MUST NOT return `Box<dyn std::error::Error>`, `String` or `&'static str` as an error type;
- MUST NOT define module-local error enums. Add a variant to the crate `Error` instead.

The only exception is a trait signature that fixes the error type, for example
`fmt::Display::fmt` returning `std::fmt::Result`, or serde's `Serialize`/`Deserialize`. In those
places the fully qualified type is used.

Modules import the alias explicitly:

```rust
use crate::errors::{Error, Result};
```

### 1.3 No error-handling crates

`thiserror`, `anyhow`, `eyre`, `snafu` and similar crates MUST NOT be added. The `Display`,
`std::error::Error` and `From` implementations are written by hand.

### 1.4 Conversions and `?`

- **Errors from external crates or std** that cross into the crate get an `impl From<X> for Error`,
  so that `?` converts them. The conversion keeps the original error as the variant's payload, so
  that `source()` can return it.
- **Domain failures**, such as an invalid locator, a malformed ADIF record or a provider rejecting
  a login, get variants with structured fields (for example
  `Provider { provider: &'static str, message: String }`), not a bare `String`, whenever the
  caller or the log needs those fields.

### 1.5 Reference shape

```rust
// src/errors.rs
use std::fmt;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Config(String),
    Provider { provider: &'static str, message: String },
    // …one variant per failure kind
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "i/o error: {e}"),
            Error::Config(msg) => write!(f, "configuration error: {msg}"),
            Error::Provider { provider, message } => write!(f, "{provider}: {message}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
```

### 1.6 Tests

`src/errors.rs` MUST have unit tests that cover:

- a non-empty `Display` for every variant;
- every `From` conversion;
- `source()` for the variants that wrap another error.

## 2. Toolchain

### 2.1 Latest stable for development

Development MUST use the latest stable Rust. `rust-toolchain.toml` selects the `stable` channel;
run `rustup update stable` before starting a feature. Plans MUST NOT pin a Rust version.

### 2.2 Pinned versions in builds

CI (`.circleci/config.yml`, pipeline parameter `rust-version`) and the `Dockerfile`
(`RUST_VERSION`) MUST pin an exact Rust version, so builds are reproducible. CI sets
`RUSTUP_TOOLCHAIN` to that version, which overrides `rust-toolchain.toml`. Raise both pins
deliberately, together, after `cargo clippy --all-targets -- -D warnings` and `cargo test` pass on
the new version.

## Changelog

| Date | Change |
|------|--------|
| 2026-10-08 | Initial version: §1 Errors and `Result`. |
| 2026-10-11 | §2 Toolchain: latest stable for development, pinned versions in CI and Docker. |

> A simple loose SQL parser for RUST

[![build](https://img.shields.io/github/actions/workflow/status/squill-app/loose-sqlparser-rs/coverage.yml?style=for-the-badge)](https://github.com/squill-app/loose-sqlparser-rs/actions/workflows/coverage.yml)
[![codecov](https://img.shields.io/codecov/c/gh/squill-app/loose-sqlparser-rs/settings/badge.svg?token=PD2KZWGW8U&style=for-the-badge&logo=codecov)](https://codecov.io/github/squill-app/loose-sqlparser-rs)

**loose-sqlparser-rs** is a non-validating SQL parser for RUST. It provides support for parsing and splitting SQL
statements.

Unlike the excellent [sqlparser](https://crates.io/crates/sqlparser) this library is not trying to build an AST from
the given input but only gives an insight of the one or many SQL statements found.

```rust
use loose_sqlparser::loose_sqlparse;

let statements: Vec<_> = loose_sqlparse("SELECT 1;SELECT 2").collect();
```

## Example

You can use the `cli` example to play with the parser and test it capabilities and limitations:

```bash
$ cargo run --example cli FILENAME.sql
```

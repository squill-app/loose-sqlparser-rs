> A simple loose SQL parser for RUST.

**loose-sqlparse** is a non-validating SQL parser for RUST. It provides support for parsing and splitting SQL
statements.

Unlike the excellent [sqlparser](https://crates.io/crates/sqlparser) this library is not trying to build an AST from
the given input but only gives an insight of the one or many SQL statements found.

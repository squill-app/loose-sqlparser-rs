> A simple loose SQL parser for RUST

[![build](https://img.shields.io/github/actions/workflow/status/squill-app/loose-sqlparse/coverage.yml?style=for-the-badge)](https://github.com/squill-app/loose-sqlparse/actions/workflows/check.yml)
[![codecov](https://img.shields.io/codecov/c/gh/squill-app/loose-sqlparse/settings/badge.svg?token=PD2KZWGW8U&style=for-the-badge&logo=codecov)](https://codecov.io/github/squill-app/loose-sqlparse)

**loose-sqlparse** is a non-validating SQL parser for RUST. It provides support for parsing and splitting SQL
statements.

Unlike the excellent [sqlparser](https://crates.io/crates/sqlparser) this library is not trying to build an AST from
the given input but only gives an insight of the one or many SQL statements found.

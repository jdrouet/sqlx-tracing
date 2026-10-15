# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Upgrade to SQLx 0.9 (breaking change); SQLx 0.8 users should stay on 0.2.x.
- Require Rust 1.94 or newer, matching SQLx 0.9.
- Adapt executor metadata methods to owned `SqlStr` values and statements.
- Preserve bind arguments, cached statements, and persistence settings when
	recording SQL text through SQLx 0.9's consuming `Execute::sql` API.

### Fixed

- Allow successive queries on traced transactions and transaction executors
	without borrowing them for the full transaction lifetime.

## [0.2.1](https://github.com/jdrouet/sqlx-tracing/compare/v0.2.0...v0.2.1) - 2026-04-26

### Added

- add MySQL support ([#4](https://github.com/jdrouet/sqlx-tracing/pull/4))
- *(pool)* Add try_begin method ([#10](https://github.com/jdrouet/sqlx-tracing/pull/10))
- add `Transaction::commit` and `Transaction::rollback` ([#5](https://github.com/jdrouet/sqlx-tracing/pull/5))

### Fixed

- relax Clone bound on Pool<DB> ([#14](https://github.com/jdrouet/sqlx-tracing/pull/14))

### Other

- bump dependencies ([#15](https://github.com/jdrouet/sqlx-tracing/pull/15))

## [0.2.0](https://github.com/jdrouet/sqlx-tracing/compare/v0.1.0...v0.2.0) - 2025-10-02

### Added

- add attributes to pool
- make sure returned_rows is populated
- trace on pool connections and transactions
- make it work with PoolConnection
- make transaction part compile
- create pool-connection and transaction

### Fixed

- unused import
- create separate builder for sqlite and postgres
- please clippy
- remove unused traits

### Other

- use opentelemetry-testing from registry
- comment the code
- update readme with pool builder
- ensure pool queries are traced
- release v0.1.0

## [0.1.0](https://github.com/jdrouet/sqlx-tracing/releases/tag/v0.1.0) - 2025-09-07

### Other

- configure for auto release
- update cargo.toml
- set versions in dev deps
- add readme
- configure
- check that it works for sqlite and postgres
- simple project

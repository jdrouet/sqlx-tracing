# sqlx-tracing

**sqlx-tracing** is a Rust library that provides OpenTelemetry-compatible tracing for SQLx database operations. It wraps SQLx connection pools and queries with tracing spans, enabling detailed observability of database interactions in distributed systems.

## Features

- **Automatic Tracing**: All SQLx queries executed through the provided pool are traced using [tracing](https://docs.rs/tracing) spans.
- **OpenTelemetry Integration**: Traces are compatible with OpenTelemetry, making it easy to export to collectors and observability platforms.
- **Error Recording**: Errors are automatically annotated with kind, message, and stacktrace in the tracing span.
- **Returned Rows**: The number of rows returned by queries is recorded for observability.
- **Database Agnostic**: Supports both PostgreSQL and SQLite via feature flags.
- **Macros**: Includes a macro for consistent span creation around queries.

## Usage

Add `sqlx-tracing` to your `Cargo.toml`:

```toml
[dependencies]
sqlx-tracing = "0.1"
sqlx = { version = "0.8", default-features = false, features = ["derive"] }
tracing = "0.1"
```

Enable the desired database feature:

- For PostgreSQL: `features = ["postgres"]`
- For SQLite: `features = ["sqlite"]`

Wrap your SQLx pool:

```rust
let pool = sqlx::PgPool::connect(&url).await?;
let traced_pool = sqlx_tracing::Pool::from(pool);
```

Use the traced pool as you would a normal SQLx pool:

```rust
let result: Option<i32> = sqlx::query_scalar("select 1")
    .fetch_optional(&traced_pool)
    .await?;
```

## OpenTelemetry Integration

To export traces, set up an OpenTelemetry collector and configure the tracing subscriber with the appropriate layers. See the `tests/common.rs` for a full example using `opentelemetry`, `opentelemetry-otlp`, and `tracing-opentelemetry`.

## Testing

Integration tests are provided for both PostgreSQL and SQLite, using [testcontainers](https://docs.rs/testcontainers) and a local OpenTelemetry collector.

## License

Licensed under MIT.

## Contributing

Contributions and issues are welcome! Please open a PR or issue on GitHub.

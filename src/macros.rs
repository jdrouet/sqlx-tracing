#[macro_export]
macro_rules! query_span {
    ($name:expr, $statement:expr) => {
        tracing::info_span!(
            $name,
            "db.system" = DB::SYSTEM,
            "db.query.text" = ?$statement,
            "db.response.returned_rows" = tracing::field::Empty,
            "error.kind" = tracing::field::Empty,
            "error.message" = tracing::field::Empty,
            "error.stacktrace" = tracing::field::Empty,
            "net.peer.name" = tracing::field::Empty,
            "net.peer.port" = tracing::field::Empty,
            "otel.kind" = "client",
            "otel.status_code" = "client",
        )
    };
}

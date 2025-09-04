impl crate::prelude::Database for sqlx::Sqlite {
    const SYSTEM: &'static str = "sqlite";
}

impl crate::prelude::DatabaseInstance for &'_ sqlx::Pool<sqlx::Sqlite> {
    fn database_name(&self) -> impl std::fmt::Display {
        self.connect_options()
            .get_filename()
            .to_string_lossy()
            .to_string()
    }
}

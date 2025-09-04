impl crate::prelude::Database for sqlx::Postgres {
    const SYSTEM: &'static str = "postgresql";
}

impl crate::prelude::DatabaseInstance for sqlx::Pool<sqlx::Postgres> {
    fn database_name(&self) -> impl std::fmt::Display {
        self.connect_options().get_host().to_owned()
    }
}

pub trait Database {
    const SYSTEM: &'static str;
}

pub trait DatabaseInstance {
    fn database_name(&self) -> impl std::fmt::Display;
}

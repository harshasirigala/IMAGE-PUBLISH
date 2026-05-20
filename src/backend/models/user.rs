#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub google_id: String,
    pub email: String,
    pub name: String,
}
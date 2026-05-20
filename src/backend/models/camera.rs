#[derive(Debug, sqlx::FromRow)]
pub struct Camera {
    pub id: i64,
    pub user_id: i64,
    pub camera_id: String,
}
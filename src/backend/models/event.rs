#[derive(Debug)]
pub struct ImageEvent {
    pub device_id: String,
    pub timestamp: String,
    pub image_url: String,
    pub bucket: String,
}
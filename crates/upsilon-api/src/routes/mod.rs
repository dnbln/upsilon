pub mod users;
pub mod repos;

#[get("/")]
pub async fn get_api_root() -> &'static str {
    "Hello world"
}
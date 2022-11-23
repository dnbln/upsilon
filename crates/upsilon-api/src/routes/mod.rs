pub mod repos;
pub mod users;

#[v1]
#[get("/")]
pub async fn get_api_root() -> &'static str {
    "Hello world"
}

api_routes!();

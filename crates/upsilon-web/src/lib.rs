use rocket::{
    async_trait,
    fairing::{Fairing, Info, Kind},
    Build, Rocket,
};

pub struct WebFairing;

#[async_trait]
impl Fairing for WebFairing {
    fn info(&self) -> Info {
        Info {
            name: "Web fairing",
            kind: Kind::Ignite | Kind::Singleton,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> rocket::fairing::Result {
        Ok(rocket)
    }
}

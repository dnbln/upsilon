#[macro_use]
extern crate rocket;

use rocket::{
    fairing::{Fairing, Info, Kind},
    Build, Rocket,
};

mod routes;

pub struct ApiFairing<const V: usize>;

macro_rules! api_fairing {
    (@version $version:literal, $($route:expr),* $(,)?) => {
        #[rocket::async_trait]
        impl Fairing for ApiFairing<$version> {
            fn info(&self) -> Info {
                Info {
                    name: concat!("API fairing (version v", $version, ")"),
                    kind: Kind::Ignite | Kind::Singleton,
                }
            }

            async fn on_ignite(&self, rocket: Rocket<Build>) -> rocket::fairing::Result {
                Ok(rocket.mount(
                    concat!("/api/v", $version),
                    routes![
                        $($route,)*
                    ],
                ))
            }
        }
    };
}

api_fairing!(
    @version 1,
    routes::get_api_root,
    routes::create_repo,
    routes::get_repo,
    routes::get_commit,
    routes::get_branch_top,
    routes::get_branch_history,
    routes::users::create_user,
);

pub struct ApiConfigurator;

#[rocket::async_trait]
impl Fairing for ApiConfigurator {
    fn info(&self) -> Info {
        Info {
            name: "API fairing configurator",
            kind: Kind::Ignite | Kind::Singleton,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> rocket::fairing::Result {
        Ok(rocket.attach(ApiFairing::<1>))
    }
}

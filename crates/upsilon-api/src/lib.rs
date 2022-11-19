#[macro_use]
extern crate rocket;
#[macro_use(v1, api_routes)]
extern crate upsilon_procx;

use rocket::{
    fairing::{Fairing, Info, Kind},
    Build, Rocket,
};

mod routes;

mod error;

pub struct ApiFairing<const V: usize>;

macro_rules! api_fairing {
    (@version $version:literal, $($routes:ty),* $(,)?) => {
        #[rocket::async_trait]
        impl Fairing for ApiFairing<$version> {
            fn info(&self) -> Info {
                Info {
                    name: concat!("API fairing (version v", $version, ")"),
                    kind: Kind::Ignite | Kind::Singleton,
                }
            }

            async fn on_ignite(&self, rocket: Rocket<Build>) -> rocket::fairing::Result {
                let mut joined = rocket::routes![];

                $(
                    joined.extend(
                        <$routes as $crate::ApiRoutes <$version>>::get_routes()
                    );
                )*

                Ok(rocket.mount(
                    concat!("/api/v", $version),
                    joined,
                ))
            }
        }
    };
}

api_fairing!(
    @version 1,
    routes::RootApi,
    routes::repos::ReposApi,
    routes::users::UsersApi,
);

pub trait ApiRoutes<const V: usize> {
    fn get_routes() -> Vec<rocket::Route>;
}

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

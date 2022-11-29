#[macro_use]
extern crate juniper;

use std::pin::Pin;

use juniper::futures::Stream;
use juniper::{futures, FieldError};

pub type Schema = juniper::RootNode<'static, QueryRoot, MutationRoot, SubscriptionRoot>;

pub struct GraphQLContext {
    db: upsilon_data::DataClientMasterHolder,
}

impl GraphQLContext {
    pub fn new(db: upsilon_data::DataClientMasterHolder) -> Self {
        Self { db }
    }
}

impl juniper::Context for GraphQLContext {}

pub struct QueryRoot;

#[graphql_object(Context = GraphQLContext)]
impl QueryRoot {
    async fn api_version() -> &str {
        "1.0"
    }
}

pub struct MutationRoot;

#[graphql_object(Context = GraphQLContext)]
impl MutationRoot {
    fn create_user() -> bool {
        true
    }
}

pub struct SubscriptionRoot;

type StringStream = Pin<Box<dyn Stream<Item = Result<String, FieldError>> + Send>>;

#[graphql_subscription(context = GraphQLContext)]
impl SubscriptionRoot {
    async fn hello_world() -> StringStream {
        let stream =
            futures::stream::iter(vec![Ok(String::from("Hello")), Ok(String::from("World!"))]);
        Box::pin(stream)
    }
}

pub struct UserRef<'a>(&'a upsilon_models::users::User);

#[graphql_object]
impl<'a> UserRef<'a> {
    fn username(&self) -> &str {
        self.0.username.as_str()
    }
}
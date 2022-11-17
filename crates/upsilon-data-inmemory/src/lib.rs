use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use upsilon_data::{async_trait, queryer_and_master_impl_trait, DataClient, DataClientQueryImpl, DataClientQueryer, CommonDataClientErrorExtractor, CommonDataClientError, DataClientQueryMaster};
use upsilon_models::users::{User, UserId};

#[derive(Debug, thiserror::Error)]
pub enum InMemoryError {
    #[error("User not found")]
    UserNotFound,
    #[error("User already exists")]
    UserAlreadyExists,
}

impl CommonDataClientErrorExtractor for InMemoryError {
    fn into_common_error(self) -> CommonDataClientError {
        CommonDataClientError::Other(Box::new(self))
    }
}

#[derive(Clone, serde::Deserialize)]
pub struct InMemoryStorageConfiguration {}

pub struct InMemoryDataStore {
    users: Arc<Mutex<BTreeMap<UserId, User>>>,
}

pub struct InMemoryDataClient(InMemoryStorageConfiguration, Box<InMemoryDataStore>);

#[async_trait]
impl DataClient for InMemoryDataClient {
    type InnerConfiguration = InMemoryStorageConfiguration;
    type Error = InMemoryError;
    type QueryImpl<'a> = InMemoryQueryImpl<'a>;

    async fn init_client(config: &Self::InnerConfiguration) -> Self::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self(config.clone(), Box::new(InMemoryDataStore {
            users: Arc::new(Mutex::new(BTreeMap::new())),
        })))
    }

    fn data_client_query_impl<'a>(&'a self) -> Self::QueryImpl<'a> {
        InMemoryQueryImpl(self)
    }
}

pub struct InMemoryQueryImpl<'a>(&'a InMemoryDataClient);

impl<'a> InMemoryQueryImpl<'a> {
    fn store(&self) -> &InMemoryDataStore {
        &self.0 .1
    }
}

#[async_trait]
impl<'a> DataClientQueryImpl<'a> for InMemoryQueryImpl<'a> {
    type Error = InMemoryError;

    async fn create_user(&self, user: User) -> Result<(), Self::Error> {
        let mut lock = self.store().users.lock().await;

        if lock.contains_key(&user.id) {
            return Err(InMemoryError::UserAlreadyExists);
        }

        lock.insert(user.id, user);

        Ok(())
    }

    async fn query_user(&self, user_id: UserId) -> Result<User, Self::Error> {
        let lock = self.store().users.lock().await;

        lock.get(&user_id)
            .map(|user| user.clone())
            .ok_or(InMemoryError::UserNotFound)
    }

    async fn set_user_name(
        &self,
        user_id: UserId,
        user_name: upsilon_models::users::Username,
    ) -> Result<(), Self::Error> {
        let mut lock = self.store().users.lock().await;

        lock.get_mut(&user_id)
            .map(|user| user.username = user_name)
            .ok_or(InMemoryError::UserNotFound)
    }

    fn as_queryer<'q>(&'q self) -> Box<dyn DataClientQueryer + 'q> {
        Box::new(InMemoryQueryer(self))
    }

    fn as_query_master<'q>(&'q self) -> Box<dyn DataClientQueryMaster + 'q> {
        Box::new(InMemoryQueryMaster(self))
    }
}

queryer_and_master_impl_trait!(InMemoryQueryer, InMemoryQueryMaster, InMemoryQueryImpl);

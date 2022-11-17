use std::collections::BTreeMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use upsilon_data::{
    async_trait, query_master_impl_trait, CommonDataClientError, CommonDataClientErrorExtractor,
    DataClient, DataClientMaster, DataClientQueryImpl, DataClientQueryMaster,
};
use upsilon_models::email::Email;
use upsilon_models::users::{User, UserId, Username};

#[derive(Debug, thiserror::Error)]
pub enum InMemoryError {
    #[error("User not found")]
    UserNotFound,
    #[error("User already exists")]
    UserAlreadyExists,
}

impl CommonDataClientErrorExtractor for InMemoryError {
    fn into_common_error(self) -> CommonDataClientError {
        match self {
            InMemoryError::UserNotFound => CommonDataClientError::UserNotFound,
            InMemoryError::UserAlreadyExists => CommonDataClientError::UserAlreadyExists,
            _ => CommonDataClientError::Other(Box::new(self)),
        }
    }
}

#[derive(Clone, Debug)]
pub enum InMemoryStorageSaveStrategy {
    Save { path: PathBuf },
    DontSave,
}

#[derive(Clone, Debug)]
pub struct InMemoryStorageConfiguration {
    pub save_strategy: InMemoryStorageSaveStrategy,
}

pub struct InMemoryDataStore {
    users: Arc<Mutex<BTreeMap<UserId, User>>>,
}

impl InMemoryDataStore {
    fn new() -> Self {
        Self {
            users: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
}

pub struct InMemoryDataClient(InMemoryStorageConfiguration, Box<InMemoryDataStore>);

#[async_trait]
impl DataClient for InMemoryDataClient {
    type InnerConfiguration = InMemoryStorageConfiguration;
    type Error = InMemoryError;
    type QueryImpl<'a> = InMemoryQueryImpl<'a>;

    async fn init_client(config: Self::InnerConfiguration) -> Self::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self(config, Box::new(InMemoryDataStore::new())))
    }

    fn data_client_query_impl<'a>(&'a self) -> Self::QueryImpl<'a> {
        InMemoryQueryImpl(self)
    }
}

#[async_trait]
impl DataClientMaster for InMemoryDataClient {
    fn query_master<'a>(&'a self) -> Box<dyn DataClientQueryMaster + 'a> {
        self.data_client_query_impl().as_query_master()
    }

    async fn on_shutdown(&self) -> Result<(), Box<dyn Error>> {
        Ok(())
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

    async fn query_user_by_username_email(
        &self,
        username_email: &str,
    ) -> Result<Option<User>, Self::Error> {
        let lock = self.store().users.lock().await;

        let user = lock
            .values()
            .find(|user| user.username == username_email || user.emails.contains(username_email));

        Ok(user.map(|user| user.clone()))
    }

    async fn set_user_name(
        &self,
        user_id: UserId,
        user_name: Username,
    ) -> Result<(), Self::Error> {
        let mut lock = self.store().users.lock().await;

        lock.get_mut(&user_id)
            .map(|user| user.username = user_name)
            .ok_or(InMemoryError::UserNotFound)
    }

    fn as_query_master(self) -> Box<dyn DataClientQueryMaster + 'a> {
        Box::new(InMemoryQueryMaster(self))
    }
}

query_master_impl_trait!(InMemoryQueryMaster, InMemoryQueryImpl);

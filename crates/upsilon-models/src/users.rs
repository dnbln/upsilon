pub mod emails;

use crate::assets::ImageAssetId;
use crate::email::Email;
use crate::users::emails::{UserEmails};


upsilon_id::id_ty!(
    #[uuid]
    #[timestamped]
    pub struct UserId;
);

crate::utils::str_newtype!(Name);
crate::utils::str_newtype!(Username);


#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub username: Username,
    pub name: Option<Name>,

    pub emails: UserEmails,
    pub avatar: Option<ImageAssetId>,
}

impl User {
    pub fn new(username: Username, primary_email: Email) -> User {
        let id = UserId::new();

        dbg!(id);

        User {
            id,
            username,
            emails: UserEmails::new(primary_email),
            avatar: None,
            name: None,
        }
    }
}

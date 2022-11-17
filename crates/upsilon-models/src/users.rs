pub mod emails;
pub mod password;

use crate::assets::ImageAssetId;
use crate::users::emails::{UserEmails};
use crate::users::password::HashedPassword;


upsilon_id::id_ty!(
    #[uuid]
    #[timestamped]
    pub struct UserId;
);

crate::utils::str_newtype!(Username);
crate::utils::str_newtype!(Name);


#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub username: Username,
    pub password: HashedPassword,
    pub name: Option<Name>,

    pub emails: UserEmails,
    pub avatar: Option<ImageAssetId>,
}

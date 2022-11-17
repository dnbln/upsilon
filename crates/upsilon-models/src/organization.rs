use crate::email::Email;
use crate::users::UserId;
upsilon_id::id_ty! {
    #[uuid]
    #[timestamped]
    pub struct OrganizationId;
}

upsilon_id::id_ty! {
    #[uuid]
    #[timestamped]
    pub struct TeamId;
}

crate::utils::str_newtype!(OrganizationName);
crate::utils::str_newtype!(OrganizationDisplayName);

#[derive(Debug, Clone)]
pub struct Organization {
    pub id: OrganizationId,
    pub owner: UserId,
    pub name: OrganizationName,
    pub display_name: Option<OrganizationDisplayName>,
    pub email: Option<Email>,
}

impl Organization {
    pub fn new(owner: UserId, name: OrganizationName) -> Organization {
        Organization {
            id: OrganizationId::new(),
            owner,
            name,
            display_name: None,
            email: None,
        }
    }

    pub fn set_display_name(&mut self, display_name: OrganizationDisplayName) {
        self.display_name = Some(display_name);
    }

    pub fn set_email(&mut self, email: Email) {
        self.email = Some(email);
    }
}

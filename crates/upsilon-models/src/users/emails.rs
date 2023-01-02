/*
 *        Copyright (c) 2022-2023 Dinu Blanovschi
 *
 *    Licensed under the Apache License, Version 2.0 (the "License");
 *    you may not use this file except in compliance with the License.
 *    You may obtain a copy of the License at
 *
 *        https://www.apache.org/licenses/LICENSE-2.0
 *
 *    Unless required by applicable law or agreed to in writing, software
 *    distributed under the License is distributed on an "AS IS" BASIS,
 *    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *    See the License for the specific language governing permissions and
 *    limitations under the License.
 */

use std::ops::Index;

use crate::email::Email;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct EmailIndex(usize);

#[derive(Debug, Clone)]
pub struct UserEmails {
    pub emails: Vec<Email>,
    pub public_email: Option<EmailIndex>,
    pub primary_email: EmailIndex,
}

crate::utils::qerror! {
    pub RemoveEmailError,
    NoSuchEmail: "no such email",
    IrremovableEmail: "irremovable email",
}

impl UserEmails {
    pub fn new(primary_email: Email) -> UserEmails {
        UserEmails {
            emails: vec![primary_email],
            public_email: None,
            primary_email: EmailIndex(0),
        }
    }

    pub fn add_email(&mut self, email: Email) {
        if self.emails.contains(&email) {
            return;
        }

        self.emails.push(email);
    }

    pub fn remove_email(&mut self, email: &Email) -> Result<(), RemoveEmailError> {
        let Some(position) = self.emails.iter().position(|it| it == email) else {
            return Err(RemoveEmailError::NoSuchEmail);
        };

        self.primary_email.email_removed(position)?;
        self.public_email.email_removed(position)?;

        self.emails.remove(position);

        Ok(())
    }

    fn get_opt(&self, index: Option<EmailIndex>) -> Option<&Email> {
        index.map(|index| &self[index])
    }

    pub fn email_index(&self, email: &Email) -> Option<EmailIndex> {
        self.emails
            .iter()
            .position(|it| it == email)
            .map(EmailIndex)
    }

    pub fn public_email(&self) -> Option<&Email> {
        self.get_opt(self.public_email)
    }

    pub fn primary_email(&self) -> &Email {
        &self[self.primary_email]
    }

    pub fn contains<T: ?Sized>(&self, email: &T) -> bool
    where
        Email: PartialEq<T>,
    {
        self.emails.iter().any(|it| *it == *email)
    }
}

impl Index<EmailIndex> for UserEmails {
    type Output = Email;

    fn index(&self, index: EmailIndex) -> &Self::Output {
        &self.emails[index.0]
    }
}

trait EmailRemoved {
    fn email_removed(&mut self, position: usize) -> Result<(), RemoveEmailError>;
}

impl EmailRemoved for Option<EmailIndex> {
    fn email_removed(&mut self, position: usize) -> Result<(), RemoveEmailError> {
        let Some(index) = self else {return Ok(());};

        if index.0 == position {
            *self = None;
        } else if index.0 > position {
            index.0 -= 1;
        }

        Ok(())
    }
}

impl EmailRemoved for EmailIndex {
    fn email_removed(&mut self, position: usize) -> Result<(), RemoveEmailError> {
        if self.0 == position {
            Err(RemoveEmailError::IrremovableEmail)?;
        } else if self.0 > position {
            self.0 -= 1;
        }

        Ok(())
    }
}

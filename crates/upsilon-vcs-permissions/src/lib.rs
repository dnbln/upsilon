/*
 *        Copyright (c) 2023 Dinu Blanovschi
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

use std::str::FromStr;

use upsilon_vcs::upsilon_git_hooks;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum GitService {
    UploadPack,
    ReceivePack,
    UploadArchive,
}

impl FromStr for GitService {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "upload-pack" => Ok(Self::UploadPack),
            "receive-pack" => Ok(Self::ReceivePack),
            "upload-archive" => Ok(Self::UploadArchive),
            _ => Err(()),
        }
    }
}

pub struct RequiredRepoPermissions {
    pub read: bool,
    pub write: bool,
}

impl RequiredRepoPermissions {
    pub fn for_service(service: GitService) -> Self {
        match service {
            GitService::UploadPack => Self {
                read: true,
                write: false,
            },
            GitService::ReceivePack => Self {
                read: true,
                write: true,
            },
            GitService::UploadArchive => Self {
                read: true,
                write: false,
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LackingPermissionsError {
    #[error("data error: {0}")]
    DataError(#[from] upsilon_data::CommonDataClientError),

    #[error("lacking read permissions")]
    Read,
    #[error("lacking write permissions")]
    Write,
}

pub async fn check_user_has_permissions(
    repo: &upsilon_models::repo::Repo,
    service: GitService,
    qm: &upsilon_data::DataQueryMaster<'_>,
    user: Option<upsilon_models::users::UserId>,
) -> Result<
    (
        upsilon_git_hooks::repo_config::RepoConfig,
        upsilon_git_hooks::user_config::UserConfig,
    ),
    LackingPermissionsError,
> {
    let required = RequiredRepoPermissions::for_service(service);

    let user_perms = if let Some(user) = user {
        let user_perms = qm.query_repo_user_perms(repo.id, user).await?;

        user_perms.unwrap_or(repo.repo_config.global_permissions)
    } else {
        repo.repo_config.global_permissions
    };

    if required.read {
        let has_read = user_perms.can_read();
        if !has_read {
            return Err(LackingPermissionsError::Read);
        }
    }

    if required.write {
        let has_write = user_perms.can_write();
        if !has_write {
            return Err(LackingPermissionsError::Write);
        }
    }

    Ok((
        upsilon_git_hooks::repo_config::RepoConfig {
            protected_branches: repo
                .repo_config
                .protected_branches
                .iter()
                .map(
                    |it| upsilon_git_hooks::repo_config::ProtectedBranchRule {
                        name: it.branch_name.clone(),
                        needs_admin: it.needs_admin,
                    },
                )
                .collect(),
        },
        upsilon_git_hooks::user_config::UserConfig {
            permissions: upsilon_git_hooks::user_config::UserPermissions {
                has_read: user_perms.can_read(),
                has_write: user_perms.can_write(),
                has_admin: user_perms.has_admin(),
            },
        },
    ))
}

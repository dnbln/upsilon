mod repo_lookup_path;

use std::fmt::Write;
use std::path::PathBuf;

use rocket::serde::json::Json;
use rocket::{tokio, State};

use upsilon_data::{CommonDataClientError, DataClientMasterHolder, DataQueryMaster};
use upsilon_models::namespace::NamespaceId;
use upsilon_models::organization::{
    Organization, OrganizationId, OrganizationName, Team, TeamId, TeamName,
};
use upsilon_models::repo::{Repo, RepoId, RepoName, RepoNamespace};
use upsilon_models::users::{User, UserId, Username};
use upsilon_vcs::{TreeWalkResult, UpsilonVcsConfig};

use crate::error::ApiResult;
use crate::routes::repos::repo_lookup_path::RepoLookupPath;

#[v1]
#[post("/repos/<repo>")]
pub async fn create_repo(
    repo: String,
    vcs_config: &State<UpsilonVcsConfig>,
    data: &State<DataClientMasterHolder>,
) -> ApiResult<String> {
    let path = vcs_config.repo_dir(&repo);

    let qm = data.query_master();

    let repo = Repo::new(
        RepoId::new(),
        RepoName::from(repo),
        RepoNamespace(NamespaceId::GlobalNamespace),
    );

    let repo_name = repo.name.clone();

    match qm.create_repo(repo).await {
        Ok(()) => {}
        Err(CommonDataClientError::RepoAlreadyExists) => {
            Err(crate::error::Error::RepoAlreadyExists)?
        }
        Err(e) => Err(e)?,
    }

    tokio::fs::create_dir_all(&path).await?;
    let _ = upsilon_vcs::init_repo_absolute(vcs_config, &path)?;

    Ok(repo_name.to_string())
}

#[v1]
#[get("/repos/<repo>")]
pub async fn get_repo(
    repo: RepoLookupPath,
    data: &State<DataClientMasterHolder>,
) -> ApiResult<String> {
    let qm = data.query_master();
    let resolved = resolve(&qm, &repo)
        .await?
        .ok_or(crate::error::Error::RepoNotFound)?;

    let resolved_path = resolved.path();

    Ok(resolved_path.display().to_string())
}

#[v1]
#[get("/repos/<repo>/branch/<branch>/top")]
pub async fn get_branch_top(
    repo: RepoLookupPath,
    branch: String,
    vcs_config: &State<UpsilonVcsConfig>,
    data: &State<DataClientMasterHolder>,
) -> ApiResult<String> {
    let qm = data.query_master();
    let resolved = resolve(&qm, &repo)
        .await?
        .ok_or(crate::error::Error::RepoNotFound)?;

    let resolved_path = resolved.path();

    let r = upsilon_vcs::get_repo(vcs_config, &resolved_path)?;
    let br = r.find_branch(&branch)?;
    let cm = br.get_commit()?;

    cm.tree()?
        .walk(upsilon_vcs::TreeWalkMode::PreOrder, |name, entry| {
            println!("{name}{}", entry.name().expect("Invalid UTF-8"));
            let id = entry.id();
            println!("{id}");

            TreeWalkResult::Ok
        })?;

    Ok(cm.displayable_message().to_string())
}

#[v1]
#[get("/repos/<repo>/branch/<branch>/history")]
pub async fn get_branch_history(
    repo: RepoLookupPath,
    branch: String,
    vcs_config: &State<UpsilonVcsConfig>,
    data: &State<DataClientMasterHolder>,
) -> ApiResult<String> {
    let qm = data.query_master();
    let resolved = resolve(&qm, &repo)
        .await?
        .ok_or(crate::error::Error::RepoNotFound)?;

    let resolved_path = resolved.path();

    let r = upsilon_vcs::get_repo(vcs_config, &resolved_path)?;
    let br = r.find_branch(&branch)?;
    let cm = br.get_commit()?;

    let mut history = String::new();

    cm.self_and_all_ascendants().try_for_each(|it| {
        let commit = it?;

        writeln!(history, "{}", commit.displayable_message()).unwrap();

        Ok::<_, upsilon_vcs::Error>(())
    })?;

    Ok(history)
}

#[v1]
#[get("/repos/<repo>/commit/<commit>")]
pub async fn get_commit(
    repo: RepoLookupPath,
    commit: String,
    vcs_config: &State<UpsilonVcsConfig>,
    data: &State<DataClientMasterHolder>,
) -> ApiResult<String> {
    let qm = data.query_master();
    let resolved = resolve(&qm, &repo)
        .await?
        .ok_or(crate::error::Error::RepoNotFound)?;

    let resolved_path = resolved.path();

    let r = upsilon_vcs::get_repo(vcs_config, &resolved_path)?;
    let cm = r.find_commit(&commit)?;

    Ok(cm.displayable_message().to_string())
}

#[derive(serde::Serialize)]
#[serde(tag = "namespace")]
pub enum RepoNsPath {
    #[serde(rename = "global")]
    GlobalNamespace { repo: RepoName, repo_id: RepoId },
    #[serde(rename = "user")]
    User {
        user: UserId,
        username: Username,
        repo: RepoName,
        repo_id: RepoId,
    },
    #[serde(rename = "organization")]
    Organization {
        organization: OrganizationId,
        organization_name: OrganizationName,
        repo: RepoName,
        repo_id: RepoId,
    },
    #[serde(rename = "team")]
    Team {
        organization: OrganizationId,
        organization_name: OrganizationName,
        team: TeamId,
        team_name: TeamName,
        repo: RepoName,
        repo_id: RepoId,
    },
}

impl From<ResolvedRepoNamespace> for RepoNsPath {
    fn from(ns: ResolvedRepoNamespace) -> Self {
        match ns {
            ResolvedRepoNamespace::GlobalNamespace(repo) => RepoNsPath::GlobalNamespace {
                repo: repo.name,
                repo_id: repo.id,
            },
            ResolvedRepoNamespace::User(user, repo) => RepoNsPath::User {
                user: user.id,
                username: user.username,
                repo: repo.name,
                repo_id: repo.id,
            },
            ResolvedRepoNamespace::Organization(org, repo) => RepoNsPath::Organization {
                organization: org.id,
                organization_name: org.name,
                repo: repo.name,
                repo_id: repo.id,
            },
            ResolvedRepoNamespace::Team(org, team, repo) => RepoNsPath::Team {
                organization: org.id,
                organization_name: org.name,
                team: team.id,
                team_name: team.name,
                repo: repo.name,
                repo_id: repo.id,
            },
        }
    }
}

#[v1]
#[get("/repos/ns/lookup/<repo_ns..>")]
pub async fn get_repo_ns_path(
    repo_ns: RepoLookupPath,
    data: &State<DataClientMasterHolder>,
) -> ApiResult<Option<Json<RepoNsPath>>> {
    let qm = data.query_master();

    let repo = resolve(&qm, &repo_ns).await?;

    Ok(repo.map(|it| Json(RepoNsPath::from(it))))
}

pub enum ResolvedRepoNamespace {
    GlobalNamespace(Repo),
    User(User, Repo),
    Organization(Organization, Repo),
    Team(Organization, Team, Repo),
}

impl ResolvedRepoNamespace {
    pub fn path(&self) -> PathBuf {
        match self {
            ResolvedRepoNamespace::GlobalNamespace(repo) => PathBuf::from(repo.name.as_str()),
            ResolvedRepoNamespace::User(user, repo) => {
                let mut path = PathBuf::from(user.username.as_str());
                path.push(repo.name.as_str());
                path
            }
            ResolvedRepoNamespace::Organization(org, repo) => {
                let mut path = PathBuf::from(org.name.as_str());
                path.push(repo.name.as_str());
                path
            }
            ResolvedRepoNamespace::Team(org, team, repo) => {
                let mut path = PathBuf::from(org.name.as_str());
                path.push(team.name.as_str());
                path.push(repo.name.as_str());
                path
            }
        }
    }
}

pub async fn resolve<'a>(
    qm: &'a DataQueryMaster<'a>,
    repo_ns: &'a RepoLookupPath,
) -> ApiResult<Option<ResolvedRepoNamespace>> {
    enum TempResolvedRepoNamespace {
        GlobalNamespace,
        User(User),
        Organization(Organization),
        Team(Organization, Team),
    }

    impl TempResolvedRepoNamespace {
        fn resolved(self, repo: Repo) -> ResolvedRepoNamespace {
            match self {
                TempResolvedRepoNamespace::GlobalNamespace => {
                    ResolvedRepoNamespace::GlobalNamespace(repo)
                }
                TempResolvedRepoNamespace::User(user) => ResolvedRepoNamespace::User(user, repo),
                TempResolvedRepoNamespace::Organization(org) => {
                    ResolvedRepoNamespace::Organization(org, repo)
                }
                TempResolvedRepoNamespace::Team(org, team) => {
                    ResolvedRepoNamespace::Team(org, team, repo)
                }
            }
        }
    }

    let mut namespace = TempResolvedRepoNamespace::GlobalNamespace;

    // get the namespace
    for i in 0..(repo_ns.len() - 1) {
        let fragment = &repo_ns[i];

        namespace = match namespace {
            TempResolvedRepoNamespace::GlobalNamespace => {
                match qm.query_organization_by_name(fragment).await? {
                    Some(org) => TempResolvedRepoNamespace::Organization(org),
                    None => match qm.query_user_by_username(fragment).await? {
                        Some(user) => TempResolvedRepoNamespace::User(user),
                        None => return Ok(None),
                    },
                }
            }
            TempResolvedRepoNamespace::User(_) => Err(crate::error::Error::ResolveImpossible)?,
            TempResolvedRepoNamespace::Organization(org) => {
                match qm.query_team_by_name(org.id, fragment).await? {
                    Some(team) => TempResolvedRepoNamespace::Team(org, team),
                    None => Err(crate::error::Error::ResolveImpossible)?,
                }
            }
            TempResolvedRepoNamespace::Team(_, _) => Err(crate::error::Error::ResolveImpossible)?,
        };
    }

    let repo_name = repo_ns.last();
    let ns_id = match &namespace {
        TempResolvedRepoNamespace::GlobalNamespace => NamespaceId::GlobalNamespace,
        TempResolvedRepoNamespace::User(user) => NamespaceId::User(user.id),
        TempResolvedRepoNamespace::Organization(org) => NamespaceId::Organization(org.id),
        TempResolvedRepoNamespace::Team(org, team) => NamespaceId::Team(org.id, team.id),
    };

    let repo = qm
        .query_repo_by_name(repo_name, &RepoNamespace(ns_id))
        .await?;

    Ok(repo.map(|repo| namespace.resolved(repo)))
}

api_routes!(ReposApi);

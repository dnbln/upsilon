/*
 *        Copyright (c) 2022 Dinu Blanovschi
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

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::future::Future;
use std::ops::Index;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use upsilon_vcs::{TreeWalkMode, TreeWalkResult};

use crate::message::Message;
use crate::private::FromFlatResponse;
use crate::refs::{BranchRef, CommitRef, SignatureKind, SignatureRef, TreeEntryRef, TreeRef};

struct Store<'r> {
    branches: Vec<upsilon_vcs::Branch<'r>>,
    commits: Vec<upsilon_vcs::Commit<'r>>,
    trees: Vec<upsilon_vcs::Tree<'r>>,
}

impl<'r> Index<CommitRef> for Store<'r> {
    type Output = upsilon_vcs::Commit<'r>;

    fn index(&self, index: CommitRef) -> &Self::Output {
        &self.commits[index.id]
    }
}

impl<'r> Index<BranchRef> for Store<'r> {
    type Output = upsilon_vcs::Branch<'r>;

    fn index(&self, index: BranchRef) -> &Self::Output {
        &self.branches[index.id]
    }
}

impl<'r> Index<TreeRef> for Store<'r> {
    type Output = upsilon_vcs::Tree<'r>;

    fn index(&self, index: TreeRef) -> &Self::Output {
        &self.trees[index.id]
    }
}

impl<'r> Store<'r> {
    fn get_sig(&self, sig: SignatureRef) -> upsilon_vcs::Signature {
        let commit = &self[sig.commit_id];

        let signature = match sig.kind {
            SignatureKind::Author => commit.author(),
            SignatureKind::Committer => commit.committer(),
        };

        signature
    }

    fn push_commit(&mut self, commit: upsilon_vcs::Commit<'r>) -> CommitRef {
        let commit_sha = commit.sha();
        if let Some(pos) = self.commits.iter().position(|c| c.sha() == commit_sha) {
            return CommitRef { id: pos };
        }

        let id = self.commits.len();
        self.commits.push(commit);
        CommitRef { id }
    }
}

pub mod branch;
pub mod commit;
pub mod signature;
pub mod tree;

pub mod message;
pub mod refs;

#[derive(Debug)]
pub enum FlatMessage {
    Branch(String),
    BranchName(BranchRef),
    BranchCommit(BranchRef),
    BranchContributors(BranchRef),
    Commit(String),
    CommitSha(CommitRef),
    CommitMessage(CommitRef),
    CommitAuthor(CommitRef),
    CommitCommitter(CommitRef),
    SignatureName(SignatureRef),
    SignatureEmail(SignatureRef),
    CommitTree(CommitRef),
    TreeEntries(TreeRef),
    WholeTreeEntries(TreeRef),

    #[doc(hidden)]
    Close,
}

#[derive(Debug)]
pub enum FlatResponse {
    Branch(BranchRef),
    BranchName(Option<String>),
    BranchContributors(BTreeMap<String, usize>),
    Commit(CommitRef),
    CommitSha(String),
    CommitMessage(Option<String>),
    CommitAuthor(SignatureRef),
    CommitCommitter(SignatureRef),
    SignatureName(Option<String>),
    SignatureEmail(Option<String>),
    CommitTree(TreeRef),
    TreeEntries(Vec<(String, TreeEntryRef)>),
    Error(upsilon_vcs::Error),

    #[doc(hidden)]
    CloseRelay,
}

struct ChannelClient {
    sender: std::sync::mpsc::SyncSender<FlatMessageAndId>,
    receiver: tokio::sync::mpsc::UnboundedReceiver<FlatResponseAndId>,
}

#[derive(Debug)]
pub struct FlatMessageAndId {
    id: u32,
    message: FlatMessage,
}

#[derive(Debug)]
pub struct FlatResponseAndId {
    id: u32,
    response: FlatResponse,
}

struct ChannelServer {
    sender: tokio::sync::mpsc::UnboundedSender<FlatResponseAndId>,
    receiver: std::sync::mpsc::Receiver<FlatMessageAndId>,
}

const CHANNEL_BUFFER_SIZE: usize = 1024;

fn new_channel() -> (ChannelClient, ChannelServer) {
    let (message_sender, message_receiver) = std::sync::mpsc::sync_channel(CHANNEL_BUFFER_SIZE);
    let (response_sender, response_receiver) = tokio::sync::mpsc::unbounded_channel();

    (
        ChannelClient {
            sender: message_sender,
            receiver: response_receiver,
        },
        ChannelServer {
            sender: response_sender,
            receiver: message_receiver,
        },
    )
}

struct ClientState {
    message_consumers: Arc<tokio::sync::Mutex<BTreeMap<u32, Box<dyn FnOnce(FlatResponse) + Send>>>>,
    index: Arc<AtomicU32>,
    sender: std::sync::mpsc::SyncSender<FlatMessageAndId>,
}

impl Drop for ClientState {
    fn drop(&mut self) {
        let _ = self.sender.send(FlatMessageAndId {
            id: 0,
            message: FlatMessage::Close,
        });
    }
}

#[derive(Clone)]
pub struct Client {
    state: Arc<ClientState>,
}

impl Client {
    fn inner(
        &self,
        receiver: tokio::sync::mpsc::UnboundedReceiver<FlatResponseAndId>,
    ) -> ClientInner {
        ClientInner {
            message_consumers: Arc::clone(&self.state.message_consumers),
            index: Arc::clone(&self.state.index),
            receiver,
        }
    }
}

struct ClientInner {
    message_consumers: Arc<tokio::sync::Mutex<BTreeMap<u32, Box<dyn FnOnce(FlatResponse) + Send>>>>,
    index: Arc<AtomicU32>,
    receiver: tokio::sync::mpsc::UnboundedReceiver<FlatResponseAndId>,
}

impl Client {
    pub async fn new<F>(repo_getter: F) -> Self
    where
        F: FnOnce() -> upsilon_vcs::Repository,
        F: Send,
        F: 'static,
    {
        let (channel_client, channel_server) = new_channel();

        let ChannelClient { receiver, sender } = channel_client;

        let client = Self {
            state: Arc::new(ClientState {
                message_consumers: Arc::new(tokio::sync::Mutex::new(BTreeMap::new())),
                index: Arc::new(AtomicU32::new(1)),
                sender,
            }),
        };

        tokio::task::spawn_blocking(move || {
            Server::init(channel_server, repo_getter()).serve();
        });

        {
            let mut client = client.inner(receiver);
            tokio::spawn(async move {
                loop {
                    if let Some(message_and_id) = client.receiver.recv().await {
                        let FlatResponseAndId { id, response } = message_and_id;

                        if let FlatResponse::CloseRelay = response {
                            break;
                        }

                        let mut message_consumers = client.message_consumers.lock().await;
                        let message_consumer = message_consumers.remove(&id).unwrap();
                        drop(message_consumers);
                        message_consumer(response);
                    }
                }
            });
        }

        client
    }

    pub async fn send<M: Message>(&self, message: M) -> M::Res {
        let id = self.state.index.fetch_add(1, Ordering::SeqCst);

        let (sender, receiver) = tokio::sync::oneshot::channel();

        {
            let mut lock = self.state.message_consumers.lock().await;

            lock.insert(
                id,
                Box::new(move |response| {
                    sender.send(response).unwrap();
                }),
            );
        }

        let message = message.to_flat_message();
        let sender = self.state.sender.clone();

        tokio::task::spawn_blocking(move || {
            // SyncSender::send is potentially blocking
            sender.send(FlatMessageAndId { id, message }).unwrap()
        });

        let response = receiver.await.unwrap();

        <M::Res as FromFlatResponse>::from_flat_response(response)
    }
}

struct Server {
    channel_server: ChannelServer,
    repo: upsilon_vcs::Repository,
}

impl Server {
    fn init(channel_server: ChannelServer, repo: upsilon_vcs::Repository) -> Self {
        Self {
            channel_server,
            repo,
        }
    }

    fn serve(mut self) {
        let mut store = Store {
            branches: Vec::new(),
            commits: Vec::new(),
            trees: Vec::new(),
        };

        while let Ok(FlatMessageAndId { id, message }) = self.channel_server.receiver.recv() {
            let response = match message {
                FlatMessage::Branch(branch_name) => {
                    let branch = self.repo.find_branch(&branch_name);
                    match branch {
                        Ok(branch) => {
                            let id = store.branches.len();
                            store.branches.push(branch);
                            FlatResponse::Branch(BranchRef { id })
                        }
                        Err(e) => FlatResponse::Error(e),
                    }
                }
                FlatMessage::BranchName(branch) => {
                    let b = &store[branch];

                    match b.name() {
                        Ok(name) => FlatResponse::BranchName(name.map(ToString::to_string)),
                        Err(e) => FlatResponse::Error(e),
                    }
                }
                FlatMessage::BranchCommit(branch) => {
                    let b = &store[branch];
                    match b.get_commit() {
                        Ok(commit) => FlatResponse::Commit(store.push_commit(commit)),
                        Err(e) => FlatResponse::Error(e),
                    }
                }
                FlatMessage::BranchContributors(branch) => {
                    fn branch_contributors<'r>(
                        store: &mut Store<'r>,
                        contributors: &mut BTreeMap<String, usize>,
                        commit: upsilon_vcs::Commit<'r>,
                    ) -> Option<upsilon_vcs::Error> {
                        let mut passed_commits = BTreeSet::new();
                        let mut commit_queue = VecDeque::new();
                        commit_queue.push_back(commit);

                        while let Some(c) = commit_queue.pop_front() {
                            if !passed_commits.insert(c.sha()) {
                                continue;
                            }

                            *contributors
                                .entry(c.author().email().unwrap_or("<invalid email>").to_string())
                                .or_insert(0) += 1;

                            for parent_commit in c.parents() {
                                commit_queue.push_back(parent_commit);
                            }
                        }

                        None
                    }

                    let b = &store[branch];
                    match b.get_commit() {
                        Ok(commit) => {
                            let mut contributors = BTreeMap::new();
                            match branch_contributors(&mut store, &mut contributors, commit) {
                                Some(e) => FlatResponse::Error(e),
                                None => FlatResponse::BranchContributors(contributors),
                            }
                        }
                        Err(err) => FlatResponse::Error(err),
                    }
                }
                FlatMessage::Commit(commit_sha) => {
                    let commit = self.repo.find_commit(&commit_sha);

                    match commit {
                        Ok(commit) => FlatResponse::Commit(store.push_commit(commit)),
                        Err(e) => FlatResponse::Error(e),
                    }
                }
                FlatMessage::CommitSha(commit_ref) => {
                    FlatResponse::CommitSha(store[commit_ref].sha())
                }
                FlatMessage::CommitMessage(commit_ref) => FlatResponse::CommitMessage(
                    store[commit_ref].message().map(ToString::to_string),
                ),
                FlatMessage::CommitAuthor(commit) => {
                    let c = &store[commit];
                    let sig = c.author();
                    FlatResponse::CommitAuthor(SignatureRef {
                        commit_id: commit,
                        kind: SignatureKind::Author,
                    })
                }
                FlatMessage::CommitCommitter(commit) => {
                    let c = &store[commit];
                    let sig = c.committer();
                    FlatResponse::CommitCommitter(SignatureRef {
                        commit_id: commit,
                        kind: SignatureKind::Committer,
                    })
                }
                FlatMessage::SignatureName(signature) => {
                    let sig = store.get_sig(signature);

                    FlatResponse::SignatureName(sig.name().map(ToString::to_string))
                }
                FlatMessage::SignatureEmail(signature) => {
                    let sig = store.get_sig(signature);

                    FlatResponse::SignatureEmail(sig.email().map(ToString::to_string))
                }
                FlatMessage::CommitTree(commit) => {
                    let c = &store[commit];

                    match c.tree() {
                        Ok(tree) => {
                            let id = store.trees.len();
                            store.trees.push(tree);
                            FlatResponse::CommitTree(TreeRef { id })
                        }
                        Err(e) => FlatResponse::Error(e),
                    }
                }
                FlatMessage::TreeEntries(tree) => {
                    let t = &store[tree];

                    let mut entries = vec![];

                    for entry in t.iter() {
                        let name = entry.name().to_string();
                        let name_clone = name.clone();

                        entries.push((
                            name_clone,
                            TreeEntryRef {
                                tree_id: tree,
                                name,
                            },
                        ))
                    }

                    FlatResponse::TreeEntries(entries)
                }
                FlatMessage::WholeTreeEntries(tree) => {
                    let t = &store[tree];

                    let mut entries = vec![];

                    match t.walk(TreeWalkMode::PreOrder, |name, entry| {
                        let e = entry.name();
                        let name = format!("{name}{e}");
                        let name_clone = name.clone();

                        entries.push((
                            name_clone,
                            TreeEntryRef {
                                tree_id: tree,
                                name,
                            },
                        ));

                        TreeWalkResult::Ok
                    }) {
                        Ok(()) => FlatResponse::TreeEntries(entries),
                        Err(e) => FlatResponse::Error(e.into()),
                    }
                }
                FlatMessage::Close => {
                    let _ = self.channel_server.sender.send(FlatResponseAndId {
                        id,
                        response: FlatResponse::CloseRelay,
                    });

                    break;
                }
            };

            self.channel_server
                .sender
                .send(FlatResponseAndId { id, response })
                .unwrap();
        }
    }
}

mod private {
    use crate::{FlatMessage, FlatResponse};

    pub trait ToFlatMessage {
        fn to_flat_message(self) -> FlatMessage;
    }

    pub trait FromFlatResponse {
        fn from_flat_response(flat_response: FlatResponse) -> Self;
    }
}

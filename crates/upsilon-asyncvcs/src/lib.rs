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

use std::collections::BTreeMap;
use std::future::Future;
use std::ops::Index;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use crate::message::Message;
use crate::private::FromFlatResponse;
use crate::refs::{BranchRef, CommitRef};

struct Repo(upsilon_vcs::Repository);

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

pub mod branch;
pub mod commit;
pub mod message;
pub mod refs;

#[derive(Debug)]
enum FlatMessage {
    Branch(String),
    Commit(String),
    CommitSha(CommitRef),
    CommitMessage(CommitRef),
}

#[derive(Debug)]
enum FlatResponse {
    Branch(BranchRef),
    Commit(CommitRef),
    CommitSha(String),
    CommitMessage(Option<String>),
    Error(upsilon_vcs::Error),
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

fn new_channel_and_server() -> (ChannelClient, ChannelServer) {
    let (message_sender, message_receiver) = std::sync::mpsc::sync_channel(1024);
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

#[derive(Clone)]
pub struct Client {
    message_consumers: Arc<tokio::sync::Mutex<BTreeMap<u32, Box<dyn FnOnce(FlatResponse) + Send>>>>,
    index: Arc<AtomicU32>,
    sender: std::sync::mpsc::SyncSender<FlatMessageAndId>,
}

impl Client {
    fn inner(
        &self,
        receiver: tokio::sync::mpsc::UnboundedReceiver<FlatResponseAndId>,
    ) -> ClientInner {
        ClientInner {
            message_consumers: Arc::clone(&self.message_consumers),
            index: Arc::clone(&self.index),
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
        let (channel_client, channel_server) = new_channel_and_server();

        let ChannelClient { receiver, sender } = channel_client;

        let client = Self {
            message_consumers: Arc::new(tokio::sync::Mutex::new(BTreeMap::new())),
            index: Arc::new(AtomicU32::new(1)),
            sender,
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
        let id = self.index.fetch_add(1, Ordering::SeqCst);

        let (sender, receiver) = tokio::sync::oneshot::channel();

        {
            let mut lock = self.message_consumers.lock().await;

            lock.insert(
                id,
                Box::new(move |response| {
                    sender.send(response).unwrap();
                }),
            );
        }

        self.sender
            .send(FlatMessageAndId {
                id,
                message: message.to_flat_message(),
            })
            .unwrap();

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
                FlatMessage::Commit(commit_sha) => {
                    let commit = self.repo.find_commit(&commit_sha);

                    match commit {
                        Ok(commit) => {
                            let id = store.commits.len();
                            store.commits.push(commit);
                            FlatResponse::Commit(CommitRef { id })
                        }
                        Err(e) => FlatResponse::Error(e),
                    }
                }
                FlatMessage::CommitSha(commit_ref) => {
                    FlatResponse::CommitSha(store[commit_ref].sha())
                }
                FlatMessage::CommitMessage(commit_ref) => FlatResponse::CommitMessage(
                    store[commit_ref].message().map(ToString::to_string),
                ),
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

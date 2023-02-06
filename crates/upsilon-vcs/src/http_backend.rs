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

use std::collections::HashMap;
use std::io::Cursor;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use std::task::{Context, Poll};

use path_slash::PathBufExt;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, ReadBuf};
use tokio::process::{Child, ChildStdout};

use crate::config::GitHttpProtocol;
use crate::UpsilonVcsConfig;

#[derive(Debug, thiserror::Error)]
pub enum HandleError {
    #[error("Disabled")]
    Disabled,
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
pub enum GitBackendCgiRequestMethod {
    Get,
    Post,
}

impl GitBackendCgiRequestMethod {
    fn as_str(&self) -> &'static str {
        match self {
            GitBackendCgiRequestMethod::Get => "GET",
            GitBackendCgiRequestMethod::Post => "POST",
        }
    }
}

#[derive(Debug)]
pub struct GitBackendCgiRequest<B: AsyncRead> {
    method: GitBackendCgiRequestMethod,
    path_info: PathBuf,
    query_string: Option<HashMap<String, String>>,
    headers: Vec<(String, String)>,
    remote_addr: SocketAddr,
    req_body: Pin<Box<B>>,
}

impl<B: AsyncRead> GitBackendCgiRequest<B> {
    pub fn new(
        method: GitBackendCgiRequestMethod,
        path_info: PathBuf,
        query_string: Option<HashMap<String, String>>,
        headers: Vec<(String, String)>,
        remote_addr: SocketAddr,
        req_body: B,
    ) -> Self {
        Self {
            method,
            path_info,
            query_string,
            headers,
            remote_addr,
            req_body: Box::pin(req_body),
        }
    }
}

enum GitBackendCgiResponseState {
    ReadbackBuffer,
    ReadbackChild,
    Done,
}

pub struct GitBackendCgiResponse {
    child: Child,
    buffer: Option<Cursor<Vec<u8>>>,

    state: GitBackendCgiResponseState,

    pub status_line: String,
    pub headers: Vec<(String, String)>,
    pub content_length: Option<usize>,
}

impl AsyncRead for GitBackendCgiResponse {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.state {
            GitBackendCgiResponseState::ReadbackBuffer => {
                let this = self.get_mut();
                let r = Cursor::poll_read(
                    Pin::new(this.buffer.as_mut().expect("Missing buffer")),
                    cx,
                    buf,
                );

                if let Poll::Ready(Ok(())) = r {
                    let len = ReadBuf::filled(buf).len();
                    if len == 0 {
                        this.state = GitBackendCgiResponseState::ReadbackChild;

                        cx.waker().wake_by_ref();
                        Poll::Pending
                    } else {
                        Poll::Ready(Ok(()))
                    }
                } else {
                    r
                }
            }
            GitBackendCgiResponseState::ReadbackChild => {
                let this = self.get_mut();
                let Some(stdout) = this.child.stdout.as_mut() else {
                    return Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, "Child has no stdout")));
                };
                let r = ChildStdout::poll_read(Pin::new(stdout), cx, buf);

                if let Poll::Ready(Ok(())) = r {
                    let len = ReadBuf::filled(buf).len();

                    if len == 0 {
                        this.state = GitBackendCgiResponseState::Done;
                    }

                    Poll::Ready(Ok(()))
                } else {
                    r
                }
            }
            GitBackendCgiResponseState::Done => Poll::Ready(Ok(())),
        }
    }
}

const BUF_SIZE: usize = 16 * 1024;

pub async fn handle<B: AsyncRead>(
    config: &UpsilonVcsConfig,
    mut req: GitBackendCgiRequest<B>,
) -> Result<GitBackendCgiResponse, HandleError> {
    let GitHttpProtocol::Enabled(_protocol_config) = &config.http_protocol else {Err(HandleError::Disabled)?};

    fn build_query_string(m: &HashMap<String, String>) -> String {
        let mut s = String::new();
        for (id, (k, v)) in m.iter().enumerate() {
            if id != 0 {
                s.push('&');
            }
            s.push_str(&format!("{k}={v}"));
        }
        s
    }

    let mut cmd = tokio::process::Command::new("git");

    cmd.arg("http-backend")
        .env(
            "GIT_PROJECT_ROOT",
            config.get_path().to_slash_lossy().as_ref(),
        )
        .env("REQUEST_METHOD", req.method.as_str())
        .env("PATH_INFO", req.path_info.to_slash_lossy().as_ref())
        .env(
            "PATH_TRANSLATED",
            config
                .get_path()
                .join(
                    // try to strip the leading `/` before joining
                    req.path_info.strip_prefix("/").unwrap_or(&req.path_info),
                )
                .to_slash_lossy()
                .as_ref(),
        )
        .env("REMOTE_ADDR", req.remote_addr.ip().to_string())
        .env(
            upsilon_git_hooks::repo_config::ENV_VAR_REPO_CONFIG,
            upsilon_git_hooks::repo_config::RepoConfig {
                protected_branches: vec!["trunk".to_string()],
            }
            .serialized(),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());

    if let Some(qs) = &req.query_string {
        cmd.env("QUERY_STRING", build_query_string(qs));
    }

    for (key, value) in &req.headers {
        let key = key.to_uppercase().replace('-', "_");
        match key.as_str() {
            "CONTENT_TYPE" | "CONTENT_LENGTH" | "REMOTE_HOST" | "REMOTE_USER" | "GIT_PROTOCOL" => {
                cmd.env(key, value);
            }
            _ => {
                cmd.env(format!("HTTP_{key}"), value);
            }
        }
    }

    let mut proc = cmd.spawn()?;

    let Some(stdin) = &mut proc.stdin else {panic!("No stdin")};
    let Some(stdout) = &mut proc.stdout else {panic!("No stdout")};

    let mut buf = [0u8; BUF_SIZE];
    let mut read_count;

    loop {
        read_count = req.req_body.read(&mut buf).await?;
        if read_count == 0 {
            break;
        }
        stdin.write_all(&buf[..read_count]).await?;
    }

    read_count = stdout.read(&mut buf).await?;

    let buf_ref = &buf[..read_count];

    const END_OF_HEADERS: &[u8] = b"\r\n\r\n";

    let headers_end = buf_ref
        .windows(END_OF_HEADERS.len())
        .position(|it| it == END_OF_HEADERS)
        .expect("Missing \\r\\n\\r\\n in HTTP response");

    // SAFETY: Headers are guaranteed to be ASCII
    #[allow(unsafe_code)]
    let headers = unsafe { std::str::from_utf8_unchecked(&buf_ref[..headers_end]) };
    let buffer = &buf_ref[headers_end + 4..]; // +4 for the actual \r\n\r\n

    let mut headers = headers
        .lines()
        .map(|line| {
            let (k, v) = line.split_once(": ").expect("Cannot split header");
            (k.to_lowercase(), v.to_string())
        })
        .collect::<Vec<_>>();

    let mut status_line = "200 OK".to_string();
    let mut content_length = None;

    for (k, v) in headers.drain_filter(|(k, _)| k == "status" || k == "content-length") {
        match k.as_str() {
            "status" => {
                status_line = v;
            }
            "content-length" => {
                content_length = Some(v.parse::<usize>().expect("Invalid content length"));
            }
            _ => unreachable!(),
        }
    }

    Ok(GitBackendCgiResponse {
        child: proc,
        headers,
        buffer: if !buffer.is_empty() {
            Some(Cursor::new(buffer.to_vec()))
        } else {
            None
        },
        state: if buffer.is_empty() {
            GitBackendCgiResponseState::ReadbackChild
        } else {
            GitBackendCgiResponseState::ReadbackBuffer
        },
        status_line,
        content_length,
    })
}

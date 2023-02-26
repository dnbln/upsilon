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

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::iter::Peekable;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::str::FromStr;

use colored::Colorize;
use log::debug;
use logos::Logos;
use rustyline::completion::{FilenameCompleter, Pair};
use rustyline::validate::{ValidationContext, ValidationResult};
use rustyline::{CompletionType, Context};
use serde::Deserialize;

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum UshParseError {
    #[error("empty")]
    Empty,
    #[error("unknown command: {0}")]
    UnknownCommand(Spanned<String>),
    #[error("unexpected token: {0}")]
    UnexpectedToken(Spanned<String>),
    #[error("expected end, got: {0}")]
    ExpectedEndOfInput(Spanned<String>),
    #[error("missing required arg: {0}")]
    MissingRequiredArg(String),
    #[error("missing required args: {0:?}")]
    MissingRequiredArgs(Vec<String>),
    #[error("expected arg, got: {0} (possible values: {1:?})")]
    ExpectedArg(Spanned<String>, Vec<String>),
    #[error("unexpected arg: {0}")]
    UnexpectedArg(Spanned<String>),
    #[error("unexpected flag: {0} (possible values: {1:?})")]
    UnexpectedFlag(Spanned<String>, Vec<String>),
    #[error("multiple flags in group '{0}': {1} and {2}")]
    FlagGroupConflict(String, Spanned<String>, Spanned<String>),
    #[error("missing required flag group flag: {0} (possible values: {1:?})")]
    MissingFlagGroupRequiredFlag(String, Vec<String>),
    #[error("expected non-empty arg, got: {0:?}")]
    EmptyArg(Spanned<String>, ArgHint),
    #[error("parse int error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
}

#[cfg(test)]
impl UshParseError {
    #[track_caller]
    fn assert_expected_end_of_input(&self, got: &str) {
        match self {
            UshParseError::ExpectedEndOfInput(Spanned { value, .. }) => {
                assert_eq!(value, got);
            }
            _ => panic!("expected ExpectedEndOfInput, got: {self:?}"),
        }
    }

    #[track_caller]
    fn assert_empty_arg(&self, arg: &str, hint: ArgHint) {
        match self {
            UshParseError::EmptyArg(Spanned { value, .. }, hint_value) => {
                assert_eq!(value, arg);
                assert_eq!(hint_value, &hint);
            }
            _ => panic!("expected EmptyArg, got: {self:?}"),
        }
    }

    #[track_caller]
    fn assert_expected_arg(&self, got: &str, possible_values: &[&str]) {
        match self {
            UshParseError::ExpectedArg(Spanned { value, .. }, values) => {
                assert_eq!(value, got);
                assert_eq!(values, possible_values);
            }
            _ => panic!("expected ExpectedArg, got: {self:?}"),
        }
    }

    #[track_caller]
    fn assert_unknown_arg(&self, unexpected: &str) {
        match self {
            UshParseError::UnexpectedArg(Spanned { value, .. }) => {
                assert_eq!(value, unexpected);
            }
            _ => panic!("expected UnexpectedArg, got: {self:?}"),
        }
    }

    #[track_caller]
    fn assert_eq_unordered<T: Debug + Ord + Clone, I: IntoIterator<Item = U>, U: Into<T>>(
        a: &[T],
        b: I,
    ) {
        let mut a = a.to_vec();
        let mut b = b.into_iter().map(Into::<T>::into).collect::<Vec<_>>();
        a.sort();
        b.sort();
        assert_eq!(a, b);
    }

    #[track_caller]
    fn assert_unknown_flag<'a, I: IntoIterator<Item = &'a str>>(
        &self,
        unexpected: &str,
        possible_values: I,
    ) {
        match self {
            UshParseError::UnexpectedFlag(Spanned { value, .. }, values) => {
                assert_eq!(value, unexpected);
                Self::assert_eq_unordered(values, possible_values);
            }
            _ => panic!("expected UnexpectedFlag, got: {self:?}"),
        }
    }
}

pub type UshParseResult<T> = Result<T, UshParseError>;

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn spanned<T>(self, value: T) -> Spanned<T> {
        Spanned::new(self, value)
    }

    pub fn spanned_string(self, value: impl Into<String>) -> Spanned<String> {
        self.spanned(value.into())
    }
}

impl Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

impl From<logos::Span> for Span {
    fn from(logos::Span { start, end }: logos::Span) -> Self {
        Self { start, end }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Spanned<T> {
    pub span: Span,
    pub value: T,
}

impl<T> Spanned<T> {
    pub fn new(span: Span, value: T) -> Self {
        Self { span, value }
    }
}

impl<T> Spanned<T>
where
    T: Deref<Target = str>,
{
    pub fn parse_to<U: FromStr>(&self) -> Result<Spanned<U>, U::Err> {
        Ok(Spanned {
            span: self.span,
            value: self.value.deref().parse()?,
        })
    }
}

impl Spanned<String> {
    pub fn cast_to<U: From<String>>(self) -> Spanned<U> {
        Spanned {
            span: self.span,
            value: self.value.into(),
        }
    }
}

impl<T: Debug> Debug for Spanned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} @ {:?}", self.value, self.span)
    }
}

impl<T: Display> Display for Spanned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CommandName(String);

macro_rules! commands {
    ($(
        $command:ident $name:literal $( aliases: [$($alias:literal),* $(,)?])?
    ),* $(,)?) => {
        #[derive(Debug, PartialEq, Eq, Clone, Copy)]
        pub enum UshCommand {
            $($command,)*
        }

        impl UshCommand {
            pub fn get(name: &str) -> Option<UshCommand> {
                match name {
                    $($name $($(| $alias)*)? => Some(UshCommand::$command),)*
                    _ => None,
                }
            }

            pub fn get_for_prefix(name: &str) -> Vec<&'static str> {
                let mut matches = Vec::new();
                $(
                    if $name.starts_with(name) {
                        matches.push($name);
                    }
                    $(
                        $(
                            if $alias.starts_with(name) {
                                matches.push($alias);
                            }
                        )*
                    )?
                )*
                matches
            }

            pub fn get_all_for_empty() -> Vec<&'static str> {
                vec![$($name),*]
            }
        }
    };
}

commands!(
    // general shell commands we will need
    Cd "cd",
    Ls "ls",
    Pwd "pwd",
    Echo "echo",
    Exit "exit",

    // upsilon-specific commands
    Login "login",
    CreateUser "create-user",
    CreateRepo "create-repo",
    Clone "clone",
    HttpUrl "http-url",
    GitUrl "git-url",
    SshUrl "ssh-url",
    Url "url",
    UploadSshKey "upload-ssh-key",
    ListUsers "list-users",
);

pub struct CompletionContext<'src> {
    line: &'src str,
    cwd: Rc<RefCell<PathBuf>>,
    usermap: Rc<RefCell<UserMap>>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UshPath(pub String);

impl From<String> for UshPath {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl UshPath {
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    pub fn deref_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl CompletionProvider for Spanned<UshPath> {
    fn lookup_element_at(
        &self,
        pos: usize,
        completion_ctx: &CompletionContext,
    ) -> Option<&dyn CompletionProvider> {
        unreachable!()
    }

    fn provide_completions(
        &self,
        pos: usize,
        at: &mut At,
        candidates: &mut Vec<Pair>,
        completion_ctx: &CompletionContext,
    ) {
        let (loc, completions) = FilenameCompleter::new()
            .complete_path(completion_ctx.line, pos)
            .unwrap();

        at.set(loc);
        candidates.extend(completions);
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UshCdCommand {
    pub command_name: Spanned<CommandName>,
    pub path: Option<Spanned<UshPath>>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UshLsCommand {
    pub command_name: Spanned<CommandName>,
    pub path: Option<Spanned<UshPath>>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UshPwdCommand {
    pub command_name: Spanned<CommandName>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UshEchoCommand {
    pub command_name: Spanned<CommandName>,
    pub args: Vec<Spanned<String>>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UshExitCommand {
    pub command_name: Spanned<CommandName>,
    pub exit_code: Option<Spanned<i32>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Username(pub String);

impl Display for Username {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for Username {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl CompletionProvider for Spanned<Username> {
    fn lookup_element_at(
        &self,
        pos: usize,
        completion_ctx: &CompletionContext,
    ) -> Option<&dyn CompletionProvider> {
        unreachable!()
    }

    fn provide_completions(
        &self,
        pos: usize,
        at: &mut At,
        candidates: &mut Vec<Pair>,
        completion_ctx: &CompletionContext,
    ) {
        let loc = self.span.start;

        let username_subslice = &completion_ctx.line[loc..pos];

        at.set(loc);
        for username in completion_ctx.usermap.borrow().map.keys() {
            if username.0.starts_with(username_subslice) {
                candidates.push(Pair {
                    display: username.0.clone(),
                    replacement: username.0.clone(),
                });
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UshLoginCommand {
    pub command_name: Spanned<CommandName>,
    pub username: Spanned<Username>,
    pub password: Spanned<String>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UshCreateUserCommand {
    pub command_name: Spanned<CommandName>,
    pub username: Spanned<Username>,
    pub password: Spanned<String>,
    pub email: Spanned<String>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UshCreateRepoCommand {
    pub command_name: Spanned<CommandName>,
    pub repo_name: Spanned<String>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UshHostInfo {
    pub hostname: String,

    pub git_http_enabled: bool,
    pub git_ssh_enabled: bool,
    pub git_protocol_enabled: bool,

    pub http_port: u16,
    pub ssh_port: u16,
    pub git_port: u16,

    pub https_enabled: bool,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UshRepoAccessProtocol {
    Git,
    Http,
    Ssh,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BuildUrlError {
    ProtocolDisabled,
}

impl UshRepoAccessProtocol {
    pub fn build_url(
        self,
        repo_path: &str,
        host_info: &UshHostInfo,
    ) -> Result<String, BuildUrlError> {
        let url = match self {
            Self::Http => {
                if !host_info.git_http_enabled {
                    return Err(BuildUrlError::ProtocolDisabled);
                }

                let (default_port, proto) = if host_info.https_enabled {
                    (443, "https")
                } else {
                    (80, "http")
                };

                match host_info.http_port {
                    port if port == default_port => {
                        format!("{proto}://{}/{repo_path}", host_info.hostname)
                    }
                    port => format!("{proto}://{}:{port}/{repo_path}", host_info.hostname),
                }
            }

            Self::Git => {
                if !host_info.git_protocol_enabled {
                    return Err(BuildUrlError::ProtocolDisabled);
                }

                match host_info.git_port {
                    9418 => format!("git://{}/{}", host_info.hostname, repo_path),
                    port => {
                        format!("git://{}:{port}/{}", host_info.hostname, repo_path)
                    }
                }
            }

            Self::Ssh => {
                if !host_info.git_ssh_enabled {
                    return Err(BuildUrlError::ProtocolDisabled);
                }

                match host_info.ssh_port {
                    22 => format!("git@{}:{}", host_info.hostname, repo_path),
                    port => format!("ssh://git@{}:{port}/{}", host_info.hostname, repo_path),
                }
            }
        };

        Ok(url)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UshCloneCommand {
    pub command_name: Spanned<CommandName>,
    pub repo_path: Spanned<String>,
    pub arg_to: Option<Spanned<UshPath>>,
    pub to: UshPath,
    pub access_protocol: UshRepoAccessProtocol,
    pub access_protocol_flag: Option<Spanned<String>>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UshHttpUrlCommand {
    pub command_name: Spanned<CommandName>,
    pub repo_path: Spanned<String>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UshGitUrlCommand {
    pub command_name: Spanned<CommandName>,
    pub repo_path: Spanned<String>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UshSshUrlCommand {
    pub command_name: Spanned<CommandName>,
    pub repo_path: Spanned<String>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UshUrlCommand {
    pub command_name: Spanned<CommandName>,
    pub repo_path: Spanned<String>,
    pub protocol: UshRepoAccessProtocol,
    pub protocol_flag: Option<Spanned<String>>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UshUploadSshKeyCommand {
    pub command_name: Spanned<CommandName>,
    pub key: Spanned<UshPath>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UshListUsersCommand {
    pub command_name: Spanned<CommandName>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum UshParsedCommand {
    Cd(UshCdCommand),
    Ls(UshLsCommand),
    Pwd(UshPwdCommand),
    Echo(UshEchoCommand),
    Exit(UshExitCommand),
    Login(UshLoginCommand),
    CreateUser(UshCreateUserCommand),
    CreateRepo(UshCreateRepoCommand),
    Clone(UshCloneCommand),
    HttpUrl(UshHttpUrlCommand),
    GitUrl(UshGitUrlCommand),
    SshUrl(UshSshUrlCommand),
    Url(UshUrlCommand),
    UploadSshKey(UshUploadSshKeyCommand),
    ListUsers(UshListUsersCommand),
}

#[cfg(test)]
impl UshParsedCommand {
    #[track_caller]
    fn assert_cd(&self, path: Option<&UshPath>) {
        match self {
            Self::Cd(cmd) => assert_eq!(cmd.path.as_ref().map(|it| &it.value), path),
            _ => panic!("expected cd command"),
        }
    }

    #[track_caller]
    fn assert_ls(&self, path: Option<&UshPath>) {
        match self {
            Self::Ls(cmd) => assert_eq!(cmd.path.as_ref().map(|it| &it.value), path),
            _ => panic!("expected ls command"),
        }
    }

    #[track_caller]
    fn assert_pwd(&self) {
        match self {
            Self::Pwd(_) => {}
            _ => panic!("expected pwd command"),
        }
    }

    #[track_caller]
    fn assert_echo(&self, args: &[&str]) {
        match self {
            Self::Echo(cmd) => {
                assert_eq!(
                    cmd.args
                        .iter()
                        .map(|it| it.value.as_str())
                        .collect::<Vec<_>>(),
                    args
                );
            }
            _ => panic!("expected echo command"),
        }
    }

    #[track_caller]
    fn assert_exit(&self, exit_code: Option<i32>) {
        match self {
            Self::Exit(UshExitCommand { exit_code: e, .. }) => {
                assert_eq!(e.map(|it| it.value), exit_code);
            }
            _ => panic!("expected exit command"),
        }
    }

    #[track_caller]
    fn assert_login(&self, username: &str, password: &str) {
        match self {
            Self::Login(cmd) => {
                assert_eq!(&cmd.username.value.0, username);
                assert_eq!(&cmd.password.value, password);
            }
            _ => panic!("expected login command"),
        }
    }

    #[track_caller]
    fn assert_create_user(&self, username: &str, password: &str, email: &str) {
        match self {
            Self::CreateUser(cmd) => {
                assert_eq!(&cmd.username.value.0, username);
                assert_eq!(&cmd.password.value, password);
                assert_eq!(&cmd.email.value, email);
            }
            _ => panic!("expected create user command"),
        }
    }

    #[track_caller]
    fn assert_create_repo(&self, repo_name: &str) {
        match self {
            Self::CreateRepo(cmd) => {
                assert_eq!(&cmd.repo_name.value, repo_name);
            }
            _ => panic!("expected create repo command"),
        }
    }

    #[track_caller]
    fn assert_clone(
        &self,
        remote_path: &str,
        to: &UshPath,
        access_protocol: UshRepoAccessProtocol,
    ) {
        match self {
            Self::Clone(cmd) => {
                assert_eq!(&cmd.repo_path.value, remote_path);
                assert_eq!(&cmd.to, to);
                assert_eq!(cmd.access_protocol, access_protocol);
            }
            _ => panic!("expected clone command"),
        }
    }

    #[track_caller]
    fn assert_http_url(&self, remote_path: &str) {
        match self {
            Self::HttpUrl(cmd) => {
                assert_eq!(&cmd.repo_path.value, remote_path);
            }
            _ => panic!("expected http url command"),
        }
    }

    #[track_caller]
    fn assert_git_url(&self, remote_path: &str) {
        match self {
            Self::GitUrl(cmd) => {
                assert_eq!(&cmd.repo_path.value, remote_path);
            }
            _ => panic!("expected git url command"),
        }
    }

    #[track_caller]
    fn assert_ssh_url(&self, remote_path: &str) {
        match self {
            Self::SshUrl(cmd) => {
                assert_eq!(&cmd.repo_path.value, remote_path);
            }
            _ => panic!("expected ssh url command"),
        }
    }

    #[track_caller]
    fn assert_url(&self, remote_path: &str, access_protocol: UshRepoAccessProtocol) {
        match self {
            Self::Url(cmd) => {
                assert_eq!(&cmd.repo_path.value, remote_path);
                assert_eq!(cmd.protocol, access_protocol);
            }
            _ => panic!("expected url command"),
        }
    }
}

struct At {
    at: Option<usize>,
}

impl At {
    fn new() -> Self {
        Self { at: None }
    }

    fn set(&mut self, at: usize) {
        if let Some(self_at) = self.at {
            if at != self_at {
                panic!("at is already set to a different value!")
            }
        }

        self.at = Some(at);
    }

    fn get(self) -> usize {
        self.at.unwrap_or(0)
    }
}

trait CompletionProvider {
    fn lookup_element_at(
        &self,
        pos: usize,
        completion_ctx: &CompletionContext,
    ) -> Option<&dyn CompletionProvider> {
        None
    }
    fn provide_completions(
        &self,
        pos: usize,
        at: &mut At,
        candidates: &mut Vec<Pair>,
        completion_ctx: &CompletionContext,
    ) {
        if let Some(element) = self.lookup_element_at(pos, completion_ctx) {
            element.provide_completions(pos, at, candidates, completion_ctx);
        }
    }
}

impl CompletionProvider for UshParsedCommand {
    fn lookup_element_at(
        &self,
        pos: usize,
        completion_ctx: &CompletionContext,
    ) -> Option<&dyn CompletionProvider> {
        match self {
            UshParsedCommand::Cd(cmd) => cmd.lookup_element_at(pos, completion_ctx),
            UshParsedCommand::Ls(cmd) => cmd.lookup_element_at(pos, completion_ctx),
            UshParsedCommand::Pwd(cmd) => cmd.lookup_element_at(pos, completion_ctx),
            UshParsedCommand::Echo(cmd) => cmd.lookup_element_at(pos, completion_ctx),
            UshParsedCommand::Exit(cmd) => cmd.lookup_element_at(pos, completion_ctx),
            UshParsedCommand::Login(cmd) => cmd.lookup_element_at(pos, completion_ctx),
            UshParsedCommand::CreateUser(cmd) => cmd.lookup_element_at(pos, completion_ctx),
            UshParsedCommand::CreateRepo(cmd) => cmd.lookup_element_at(pos, completion_ctx),
            UshParsedCommand::Clone(cmd) => cmd.lookup_element_at(pos, completion_ctx),
            UshParsedCommand::HttpUrl(cmd) => cmd.lookup_element_at(pos, completion_ctx),
            UshParsedCommand::GitUrl(cmd) => cmd.lookup_element_at(pos, completion_ctx),
            UshParsedCommand::SshUrl(cmd) => cmd.lookup_element_at(pos, completion_ctx),
            UshParsedCommand::Url(cmd) => cmd.lookup_element_at(pos, completion_ctx),
            UshParsedCommand::UploadSshKey(cmd) => cmd.lookup_element_at(pos, completion_ctx),
            UshParsedCommand::ListUsers(cmd) => cmd.lookup_element_at(pos, completion_ctx),
        }
    }
}

impl CompletionProvider for UshCdCommand {
    fn lookup_element_at(
        &self,
        pos: usize,
        completion_ctx: &CompletionContext,
    ) -> Option<&dyn CompletionProvider> {
        let Some(path) = self.path.as_ref() else {
            return None;
        };

        if path.span.start <= pos && pos <= path.span.end {
            return Some(path as &dyn CompletionProvider);
        }

        None
    }
}

impl CompletionProvider for UshLsCommand {
    fn lookup_element_at(
        &self,
        pos: usize,
        completion_ctx: &CompletionContext,
    ) -> Option<&dyn CompletionProvider> {
        let Some(path) = self.path.as_ref() else {
            return None;
        };

        if path.span.start <= pos && pos <= path.span.end {
            return Some(path as &dyn CompletionProvider);
        }

        None
    }
}

impl CompletionProvider for UshPwdCommand {}
impl CompletionProvider for UshEchoCommand {}
impl CompletionProvider for UshExitCommand {}

impl CompletionProvider for UshLoginCommand {
    fn lookup_element_at(
        &self,
        pos: usize,
        completion_ctx: &CompletionContext,
    ) -> Option<&dyn CompletionProvider> {
        if self.username.span.start <= pos && pos <= self.username.span.end {
            return Some(&self.username as &dyn CompletionProvider);
        }

        None
    }
}

impl CompletionProvider for UshCreateUserCommand {}
impl CompletionProvider for UshCreateRepoCommand {}
impl CompletionProvider for UshCloneCommand {}
impl CompletionProvider for UshHttpUrlCommand {}
impl CompletionProvider for UshGitUrlCommand {}
impl CompletionProvider for UshSshUrlCommand {}
impl CompletionProvider for UshUrlCommand {}
impl CompletionProvider for UshUploadSshKeyCommand {
    fn lookup_element_at(
        &self,
        pos: usize,
        completion_ctx: &CompletionContext,
    ) -> Option<&dyn CompletionProvider> {
        if self.key.span.start <= pos && pos <= self.key.span.end {
            return Some(&self.key as &dyn CompletionProvider);
        }

        None
    }
}

impl CompletionProvider for UshListUsersCommand {}

#[derive(logos::Logos, Debug, PartialEq, Eq)]
pub enum Token {
    #[regex(r"[~a-zA-Z0-9_\.\\\\/_:\-]+")]
    Value,

    #[regex(r"'[^']*'")]
    SingleQuotedValue,

    #[regex(r#""([^"\\]*|(\\\\)*\\")*""#)]
    DoubleQuotedValue,

    #[error]
    #[regex(r"[ \t\f]+", logos::skip)]
    Error,
}

struct UshCommandLexerImpl<'src> {
    lexer: logos::Lexer<'src, Token>,
}

impl<'src> UshCommandLexerImpl<'src> {
    pub fn new(line: &'src str) -> Self {
        Self {
            lexer: Token::lexer(line),
        }
    }
}

impl<'src> Iterator for UshCommandLexerImpl<'src> {
    type Item = (Token, Span, &'src str);

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.lexer.next()?;

        let span = self.lexer.span();
        let slice = self.lexer.slice();

        Some((token, Span::from(span), slice))
    }
}

pub struct UshCommandLexer<'src>(Peekable<UshCommandLexerImpl<'src>>);

impl<'src> UshCommandLexer<'src> {
    pub fn new(line: &'src str) -> Self {
        Self(UshCommandLexerImpl::new(line).peekable())
    }

    pub fn peek(&mut self) -> Option<&(Token, Span, &'src str)> {
        self.0.peek()
    }
}

impl<'src> Iterator for UshCommandLexer<'src> {
    type Item = (Token, Span, &'src str);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

pub struct UshCommandParser<'src> {
    lexer: UshCommandLexer<'src>,
    command: UshCommand,
    command_name: Spanned<CommandName>,
}

impl<'src> UshCommandParser<'src> {
    const GIT: &'static str = "git";
    const HTTP: &'static str = "http";
    const PROTO: &'static str = "protocol";
    const SSH: &'static str = "ssh";

    fn new(line: &'src str) -> UshParseResult<Self> {
        let mut lexer = UshCommandLexer::new(line);

        let Some((token, span, slice)) = lexer.next() else {return Err(UshParseError::Empty)};
        let Token::Value = token else {return Err(UshParseError::UnexpectedToken(span.spanned_string(slice)));};

        let command = UshCommand::get(slice)
            .ok_or(UshParseError::UnknownCommand(span.spanned_string(slice)))?;

        let command_name = slice.to_string();

        let command_name = Spanned {
            span,
            value: CommandName(command_name),
        };

        let parser = Self {
            lexer,
            command,
            command_name,
        };

        Ok(parser)
    }

    fn opt_path(&mut self) -> Option<Spanned<UshPath>> {
        let (token, span, slice) = self.lexer.peek()?;

        if !matches!(
            token,
            Token::Value | Token::SingleQuotedValue | Token::DoubleQuotedValue
        ) {
            return None;
        }

        let span = *span;
        let path = UshPath::new(slice.to_string());

        self.lexer.next();

        Some(Spanned { span, value: path })
    }

    fn expect_end(&mut self) -> UshParseResult<()> {
        let Some((_token, span, slice)) = self.lexer.next() else {return Ok(())};
        Err(UshParseError::ExpectedEndOfInput(
            span.spanned_string(slice),
        ))
    }

    fn fully_parse(mut self) -> UshParseResult<UshParsedCommand> {
        let parsed = match self.command {
            UshCommand::Cd => UshParsedCommand::Cd(self.parse_cd()?),
            UshCommand::Ls => UshParsedCommand::Ls(self.parse_ls()?),
            UshCommand::Pwd => UshParsedCommand::Pwd(self.parse_pwd()?),
            UshCommand::Echo => UshParsedCommand::Echo(self.parse_echo()?),
            UshCommand::Exit => UshParsedCommand::Exit(self.parse_exit()?),
            UshCommand::Login => UshParsedCommand::Login(self.parse_login()?),
            UshCommand::CreateUser => UshParsedCommand::CreateUser(self.parse_create_user()?),
            UshCommand::CreateRepo => UshParsedCommand::CreateRepo(self.parse_create_repo()?),
            UshCommand::Clone => UshParsedCommand::Clone(self.parse_clone()?),
            UshCommand::HttpUrl => UshParsedCommand::HttpUrl(self.parse_http_url()?),
            UshCommand::GitUrl => UshParsedCommand::GitUrl(self.parse_git_url()?),
            UshCommand::SshUrl => UshParsedCommand::SshUrl(self.parse_ssh_url()?),
            UshCommand::Url => UshParsedCommand::Url(self.parse_url()?),
            UshCommand::UploadSshKey => {
                UshParsedCommand::UploadSshKey(self.parse_upload_ssh_key()?)
            }
            UshCommand::ListUsers => UshParsedCommand::ListUsers(self.parse_list_users()?),
        };

        self.expect_end()?;

        Ok(parsed)
    }

    fn parse_cd(&mut self) -> UshParseResult<UshCdCommand> {
        let path = self.opt_path();

        Ok(UshCdCommand {
            command_name: self.command_name.clone(),
            path,
        })
    }

    fn parse_ls(&mut self) -> UshParseResult<UshLsCommand> {
        let path = self.opt_path();

        Ok(UshLsCommand {
            command_name: self.command_name.clone(),
            path,
        })
    }

    fn parse_pwd(&mut self) -> UshParseResult<UshPwdCommand> {
        Ok(UshPwdCommand {
            command_name: self.command_name.clone(),
        })
    }

    fn parse_echo(&mut self) -> UshParseResult<UshEchoCommand> {
        let mut args = Vec::new();

        for (token, span, slice) in self.lexer.by_ref() {
            if token == Token::Error {
                return Err(UshParseError::UnexpectedToken(span.spanned_string(slice)));
            }

            let arg = Spanned {
                span,
                value: slice.to_string(),
            };

            args.push(arg);
        }

        Ok(UshEchoCommand {
            command_name: self.command_name.clone(),
            args,
        })
    }

    fn parse_exit(&mut self) -> UshParseResult<UshExitCommand> {
        let next = self.lexer.next();

        let exit_code = if let Some((Token::Value, span, slice)) = next {
            Some(Spanned {
                span,
                value: slice.parse()?,
            })
        } else {
            None
        };

        Ok(UshExitCommand {
            command_name: self.command_name.clone(),
            exit_code,
        })
    }

    fn parse_login(&mut self) -> UshParseResult<UshLoginCommand> {
        const USERNAME: &str = "username";
        const PASSWORD: &str = "password";

        let arg_store = ArgDeclList::new()
            .arg(USERNAME, |decl| decl.hint(ArgHint::Username))
            .arg(PASSWORD, |decl| decl)
            .parse_from(self)?;

        let username = arg_store.required_arg(USERNAME)?.cast_to();
        let password = arg_store.required_arg(PASSWORD)?;

        Ok(UshLoginCommand {
            command_name: self.command_name.clone(),
            username,
            password,
        })
    }

    fn parse_create_user(&mut self) -> UshParseResult<UshCreateUserCommand> {
        const USERNAME: &str = "username";
        const PASSWORD: &str = "password";
        const EMAIL: &str = "email";

        let arg_store = ArgDeclList::new()
            .arg(USERNAME, |decl| decl)
            .arg(PASSWORD, |decl| decl)
            .arg(EMAIL, |decl| decl)
            .parse_from(self)?;

        let username = arg_store.required_arg(USERNAME)?.cast_to();
        let password = arg_store.required_arg(PASSWORD)?;
        let email = arg_store.required_arg(EMAIL)?;

        Ok(UshCreateUserCommand {
            command_name: self.command_name.clone(),
            username,
            password,
            email,
        })
    }

    fn parse_create_repo(&mut self) -> UshParseResult<UshCreateRepoCommand> {
        const NAME: &str = "name";

        let arg_store = ArgDeclList::new().arg(NAME, |decl| decl).parse_from(self)?;

        let name = arg_store.required_arg(NAME)?.cast_to();

        Ok(UshCreateRepoCommand {
            command_name: self.command_name.clone(),
            repo_name: name,
        })
    }

    fn proto_group_fg_init(fg: FlagGroup) -> FlagGroup {
        fg.flag(Self::GIT, |decl| decl)
            .flag(Self::SSH, |decl| decl)
            .flag(Self::HTTP, |decl| decl)
            .required()
            .group_default(Self::HTTP)
    }

    fn proto_group() -> (
        &'static str,
        fn(FlagGroup) -> FlagGroup,
        fn(FlagValue) -> (UshRepoAccessProtocol, Option<Spanned<String>>),
    ) {
        (Self::PROTO, Self::proto_group_fg_init, |v| match v {
            FlagValue::Present(spanned, true) if spanned.value == "--git" => {
                (UshRepoAccessProtocol::Git, Some(spanned))
            }
            FlagValue::Present(spanned, true) if spanned.value == "--ssh" => {
                (UshRepoAccessProtocol::Ssh, Some(spanned))
            }
            FlagValue::Present(spanned, true) if spanned.value == "--http" => {
                (UshRepoAccessProtocol::Http, Some(spanned))
            }
            FlagValue::Default(name, true) if name == "git" => (UshRepoAccessProtocol::Git, None),
            FlagValue::Default(name, true) if name == "ssh" => (UshRepoAccessProtocol::Ssh, None),
            FlagValue::Default(name, true) if name == "http" => (UshRepoAccessProtocol::Http, None),
            _ => unreachable!(),
        })
    }

    fn parse_clone(&mut self) -> UshParseResult<UshCloneCommand> {
        const REPO_PATH: &str = "remote-path";
        const TO: &str = "to";

        let (protocol, proto_group_fg_init, proto_group_value) = Self::proto_group();

        let arg_store = ArgDeclList::new()
            .arg(REPO_PATH, |decl| decl)
            .arg(TO, |decl| decl.optional())
            .flag_group(protocol, proto_group_fg_init)
            .parse_from(self)?;

        let (access_protocol, access_protocol_flag) =
            proto_group_value(arg_store.flag_group_required(protocol));

        let repo_path = arg_store.required_arg(REPO_PATH)?;
        let repo_name = repo_path.value.split('/').last().unwrap();
        let arg_to = arg_store.optional_arg(TO).map(Spanned::cast_to::<UshPath>);
        let to = arg_to
            .as_ref()
            .map_or_else(|| UshPath::new(repo_name), |to| to.value.clone());

        Ok(UshCloneCommand {
            command_name: self.command_name.clone(),
            repo_path,
            arg_to,
            to,
            access_protocol,
            access_protocol_flag,
        })
    }

    fn parse_git_url(&mut self) -> UshParseResult<UshGitUrlCommand> {
        const REPO_PATH: &str = "remote-path";

        let arg_store = ArgDeclList::new()
            .arg(REPO_PATH, |decl| decl)
            .parse_from(self)?;

        let repo_path = arg_store.required_arg(REPO_PATH)?;

        Ok(UshGitUrlCommand {
            command_name: self.command_name.clone(),
            repo_path,
        })
    }

    fn parse_http_url(&mut self) -> UshParseResult<UshHttpUrlCommand> {
        const REPO_PATH: &str = "remote-path";

        let arg_store = ArgDeclList::new()
            .arg(REPO_PATH, |decl| decl)
            .parse_from(self)?;

        let repo_path = arg_store.required_arg(REPO_PATH)?;

        Ok(UshHttpUrlCommand {
            command_name: self.command_name.clone(),
            repo_path,
        })
    }

    fn parse_ssh_url(&mut self) -> UshParseResult<UshSshUrlCommand> {
        const REPO_PATH: &str = "remote-path";

        let arg_store = ArgDeclList::new()
            .arg(REPO_PATH, |decl| decl)
            .parse_from(self)?;

        let repo_path = arg_store.required_arg(REPO_PATH)?;

        Ok(UshSshUrlCommand {
            command_name: self.command_name.clone(),
            repo_path,
        })
    }

    fn parse_url(&mut self) -> UshParseResult<UshUrlCommand> {
        const REPO_PATH: &str = "remote-path";

        let (proto, proto_group_fg_init, proto_group_value) = Self::proto_group();
        let arg_store = ArgDeclList::new()
            .arg(REPO_PATH, |decl| decl)
            .flag_group(proto, proto_group_fg_init)
            .parse_from(self)?;

        let (protocol, protocol_flag) = proto_group_value(arg_store.flag_group_required(proto));

        let repo_path = arg_store.required_arg(REPO_PATH)?;

        Ok(UshUrlCommand {
            command_name: self.command_name.clone(),
            repo_path,
            protocol,
            protocol_flag,
        })
    }

    fn parse_upload_ssh_key(&mut self) -> UshParseResult<UshUploadSshKeyCommand> {
        const KEY: &str = "key";

        let arg_store = ArgDeclList::new()
            .arg(KEY, |decl| decl.hint(ArgHint::Path))
            .parse_from(self)?;

        let key = arg_store.required_arg(KEY)?.cast_to();

        Ok(UshUploadSshKeyCommand {
            command_name: self.command_name.clone(),
            key,
        })
    }

    fn parse_list_users(&mut self) -> UshParseResult<UshListUsersCommand> {
        Ok(UshListUsersCommand {
            command_name: self.command_name.clone(),
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ArgHint {
    None,
    Path,
    Username,
}

pub struct ArgDecl {
    name: Cow<'static, str>,
    required: bool,
    default: Option<String>,
    allow_empty: bool,
    hint: ArgHint,
}

impl ArgDecl {
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            required: true,
            default: None,
            allow_empty: false,
            hint: ArgHint::None,
        }
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }

    pub fn allow_empty(mut self) -> Self {
        self.allow_empty = true;
        self
    }

    pub fn hint(mut self, hint: ArgHint) -> Self {
        self.hint = hint;
        self
    }
}

pub struct FlagDecl {
    name: Cow<'static, str>,
    negative_prefix: Option<Cow<'static, str>>,
    default: Option<bool>,
}

impl FlagDecl {
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            negative_prefix: None,
            default: None,
        }
    }

    pub fn negative_prefix(mut self, prefix: impl Into<Cow<'static, str>>) -> Self {
        self.negative_prefix = Some(prefix.into());
        self
    }

    pub fn allow_negative(mut self) -> Self {
        self.negative_prefix = Some("no-".into());
        self
    }

    pub fn default(mut self, default: bool) -> Self {
        self.default = Some(default);
        self
    }
}

pub struct FlagGroup {
    group_name: Cow<'static, str>,
    flags: Vec<FlagDecl>,
    required: bool,
    default: Option<Cow<'static, str>>,
}

impl FlagGroup {
    fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            group_name: name.into(),
            flags: Vec::new(),
            required: false,
            default: None,
        }
    }

    pub fn flag<F>(mut self, name: impl Into<Cow<'static, str>>, f: F) -> Self
    where
        F: FnOnce(FlagDecl) -> FlagDecl,
    {
        let flag = f(FlagDecl::new(name));
        if flag.default.is_some() || flag.negative_prefix.is_some() {
            panic!("default and negative flags are not allowed in flag groups");
        }
        self.flags.push(flag);
        self
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn group_default(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.default = Some(name.into());
        self
    }

    fn contains_flag(&self, name: &str) -> bool {
        self.flags.iter().any(|f| f.name == name)
    }
}

#[derive(Default)]
pub struct ArgDeclList {
    args: Vec<ArgDecl>,
    flags: Vec<FlagDecl>,
    flag_groups: Vec<FlagGroup>,
}

impl ArgDeclList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn arg<F>(mut self, name: impl Into<Cow<'static, str>>, decl: F) -> Self
    where
        F: FnOnce(ArgDecl) -> ArgDecl,
    {
        let decl = decl(ArgDecl::new(name));
        self.args.push(decl);
        self
    }

    pub fn flag<F>(mut self, name: impl Into<Cow<'static, str>>, decl: F) -> Self
    where
        F: FnOnce(FlagDecl) -> FlagDecl,
    {
        let decl = decl(FlagDecl::new(name));
        self.flags.push(decl);
        self
    }

    pub fn flag_group<F>(mut self, name: impl Into<Cow<'static, str>>, decl: F) -> Self
    where
        F: FnOnce(FlagGroup) -> FlagGroup,
    {
        let decl = decl(FlagGroup::new(name));
        self.flag_groups.push(decl);
        self
    }

    fn arg_names(&self) -> Vec<String> {
        self.args
            .iter()
            .map(|decl| decl.name.to_string())
            .collect::<Vec<_>>()
    }

    fn all_flag_names(&self) -> Vec<String> {
        self.flags
            .iter()
            .chain(self.flag_groups.iter().flat_map(|group| group.flags.iter()))
            .flat_map(|decl| {
                let mut v = Vec::with_capacity(2);
                v.push(decl.name.to_string());
                v.extend(
                    decl.negative_prefix
                        .iter()
                        .map(|prefix| format!("{}{}", prefix, decl.name)),
                );

                v
            })
            .collect::<Vec<_>>()
    }

    fn parse_arg<'a>(&self, span: Span, slice: &'a str) -> UshParseResult<(&'a str, ArgValue)> {
        let (name, value) = slice.split_once(':').ok_or_else(|| {
            UshParseError::ExpectedArg(span.spanned_string(slice), self.arg_names())
        })?;

        let value_span_start = span.start + name.len() + 1;
        let value_span = Span {
            start: value_span_start,
            ..span
        };

        let decl = self
            .args
            .iter()
            .find(|decl| decl.name == name)
            .ok_or_else(|| UshParseError::UnexpectedArg(span.spanned_string(name)))?;

        if value.is_empty() && !decl.allow_empty {
            return Err(UshParseError::EmptyArg(
                span.spanned_string(slice),
                decl.hint,
            ));
        }

        Ok((
            name,
            ArgValue::Value(value_span.spanned_string(value), span),
        ))
    }

    fn parse_flag(&self, span: Span, slice: &str) -> UshParseResult<(&str, FlagValue)> {
        let Some(flag) = slice.strip_prefix("--") else {
            unreachable!("Shouldn't have called parse_flag with a non-flag, see parse_from");
        };

        let (decl, flag_value) = self
            .flags
            .iter()
            .chain(self.flag_groups.iter().flat_map(|group| group.flags.iter()))
            .find_map(|decl| {
                if decl.name == flag {
                    Some((decl, true))
                } else if decl
                    .negative_prefix
                    .as_ref()
                    .map_or(false, |prefix| format!("{prefix}{}", decl.name) == flag)
                {
                    Some((decl, false))
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                UshParseError::UnexpectedFlag(span.spanned_string(slice), self.all_flag_names())
            })?;

        Ok((
            decl.name.as_ref(),
            FlagValue::Present(span.spanned_string(slice), flag_value),
        ))
    }

    pub fn parse_from(self, parser: &mut UshCommandParser) -> UshParseResult<ArgStore> {
        let mut args = ArgStore::new();

        for decl in &self.args {
            if let Some(default) = &decl.default {
                args.args
                    .insert(decl.name.to_string(), ArgValue::Default(default.clone()));
            }
        }

        for decl in &self.flags {
            if let Some(default) = decl.default {
                args.flags.insert(
                    decl.name.to_string(),
                    FlagValue::Default(decl.name.to_string(), default),
                );
            }
        }

        for flag_group in &self.flag_groups {
            for decl in &flag_group.flags {
                if let Some(default) = decl.default {
                    args.flags.insert(
                        decl.name.to_string(),
                        FlagValue::Default(decl.name.to_string(), default),
                    );
                }
            }

            if let Some(default) = &flag_group.default {
                args.flags.insert(
                    default.to_string(),
                    FlagValue::Default(default.to_string(), true),
                );
            }
        }

        for (token, span, slice) in &mut parser.lexer {
            match token {
                Token::Error => {
                    return Err(UshParseError::UnexpectedToken(span.spanned_string(slice)));
                }
                Token::Value => {
                    if slice.starts_with("--") {
                        let (name, flag_value) = self.parse_flag(span, slice)?;

                        args.flags.insert(name.to_string(), flag_value);
                    } else {
                        let (name, value) = self.parse_arg(span, slice)?;

                        args.args.insert(name.to_string(), value);
                    }
                }
                t => {
                    return Err(UshParseError::UnexpectedToken(span.spanned_string(slice)));
                }
            }
        }

        let missing_required_args = self
            .args
            .iter()
            .filter(|decl| decl.required && !args.args.contains_key(decl.name.as_ref()))
            .map(|decl| decl.name.to_string())
            .collect::<Vec<_>>();

        if !missing_required_args.is_empty() {
            return Err(UshParseError::MissingRequiredArgs(missing_required_args));
        }

        for flag_group in &self.flag_groups {
            let mut flag_group_value = None::<(String, FlagValue)>;
            for (flag, value) in &args.flags {
                if flag_group.contains_flag(flag) {
                    match value {
                        FlagValue::Present(..) => {
                            if let Some((_old_flag, old_value @ FlagValue::Present(..))) =
                                flag_group_value
                            {
                                return Err(UshParseError::FlagGroupConflict(
                                    flag_group.group_name.to_string(),
                                    old_value.unwrap_present_spanned().clone(),
                                    value.unwrap_present_spanned().clone(),
                                ));
                            } else {
                                flag_group_value = Some((flag.clone(), value.clone()));
                            }
                        }
                        FlagValue::Default(..) => {
                            if flag_group_value.is_none() {
                                flag_group_value = Some((flag.clone(), value.clone()));
                            }
                        }
                    }
                }
            }

            if let Some((_flag, value)) = flag_group_value {
                args.flag_groups
                    .insert(flag_group.group_name.to_string(), value);
            } else if flag_group.required {
                return Err(UshParseError::MissingFlagGroupRequiredFlag(
                    flag_group.group_name.to_string(),
                    flag_group
                        .flags
                        .iter()
                        .map(|decl| decl.name.to_string())
                        .collect(),
                ));
            }
        }

        Ok(args)
    }
}

pub enum ArgValue {
    Value(Spanned<String>, Span),
    Default(String),
}

impl ArgValue {
    fn arg_str(&self) -> &str {
        match self {
            ArgValue::Value(s, ..) => &s.value,
            ArgValue::Default(s) => s,
        }
    }

    fn span(&self) -> Option<Span> {
        match self {
            ArgValue::Value(s, ..) => Some(s.span),
            ArgValue::Default(s) => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum FlagValue {
    Present(Spanned<String>, bool),
    Default(String, bool),
}

impl FlagValue {
    fn value(&self) -> bool {
        match self {
            FlagValue::Present(_, v) => *v,
            FlagValue::Default(_, v) => *v,
        }
    }

    fn unwrap_present_spanned(&self) -> &Spanned<String> {
        match self {
            FlagValue::Present(s, ..) => s,
            FlagValue::Default(..) => panic!("unwrap_present_spanned called on FlagValue::Default"),
        }
    }
}

#[derive(Default)]
pub struct ArgStore {
    args: HashMap<String, ArgValue>,
    flags: HashMap<String, FlagValue>,
    flag_groups: HashMap<String, FlagValue>,
}

impl ArgStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn arg_str(&self, name: &str) -> Option<&str> {
        self.args.get(name).map(|s| s.arg_str())
    }

    pub fn arg_parse<T: FromStr>(&self, name: &str) -> Option<Result<T, T::Err>> {
        self.arg_str(name).map(str::parse)
    }

    pub fn arg_span(&self, name: &str) -> Option<Span> {
        self.args.get(name).and_then(|s| s.span())
    }

    pub fn arg(&self, name: &str) -> Option<&ArgValue> {
        self.args.get(name)
    }

    pub fn required_arg(&self, name: &str) -> UshParseResult<Spanned<String>> {
        match self.arg(name) {
            Some(ArgValue::Value(s, ..)) => Ok(s.clone()),
            Some(ArgValue::Default(s)) => {
                panic!("Required arg {name} has default value {s}")
            }
            None => Err(UshParseError::MissingRequiredArg(name.to_string())),
        }
    }

    pub fn optional_arg(&self, name: &str) -> Option<Spanned<String>> {
        match self.arg(name) {
            Some(ArgValue::Value(s, ..)) => Some(s.clone()),
            Some(ArgValue::Default(s)) => {
                panic!("Optional arg {name} has default value {s}")
            }
            None => None,
        }
    }

    pub fn flag_opt(&self, name: &str) -> Option<bool> {
        self.flags.get(name).map(|it| it.value())
    }

    pub fn flag(&self, name: &str) -> bool {
        self.flag_opt(name)
            .expect("flag should have been set with default value; unknown flag")
    }

    pub fn flag_group_required(&self, name: &str) -> FlagValue {
        self.flag_groups
            .get(name)
            .cloned()
            .expect("flag group should have been set with default value (or previous error handling didn't catch the error); unknown flag group")
    }
}

pub fn parse_line(line: &str) -> UshParseResult<UshParsedCommand> {
    UshCommandParser::new(line).and_then(UshCommandParser::fully_parse)
}

pub struct Helper {
    pub cwd: Rc<RefCell<PathBuf>>,
    pub usermap: Rc<RefCell<UserMap>>,
}

impl rustyline::completion::Completer for Helper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let result = parse_line(line);

        let parsed = match result {
            Ok(parsed) => parsed,
            Err(UshParseError::ExpectedArg(token, names)) => {
                debug_assert!(!token.value.contains(':'));
                return if token.span.end == pos {
                    let name_token = &token.value;
                    let mut candidates = Vec::new();

                    for name in names {
                        if name.starts_with(name_token) {
                            candidates.push(Pair {
                                replacement: format!("{name}:"),
                                display: name,
                            });
                        }
                    }

                    Ok((token.span.start, candidates))
                } else {
                    Ok((0, Vec::with_capacity(0)))
                };
            }
            Err(UshParseError::MissingRequiredArgs(args)) => {
                debug_assert!(!line.is_empty());

                if !Self::pos_around_ws(line, pos) {
                    return Ok((0, Vec::with_capacity(0)));
                }

                return Ok((
                    pos,
                    args.into_iter()
                        .map(|arg| Pair {
                            replacement: format!("{arg}:"),
                            display: arg,
                        })
                        .collect(),
                ));
            }
            Err(UshParseError::UnknownCommand(name)) => {
                return Ok((
                    name.span.start,
                    UshCommand::get_for_prefix(&name.value)
                        .into_iter()
                        .map(|it| Pair {
                            replacement: it.to_string(),
                            display: it.to_string(),
                        })
                        .collect(),
                ))
            }
            Err(UshParseError::Empty) => {
                return Ok((
                    0,
                    UshCommand::get_all_for_empty()
                        .into_iter()
                        .map(|it| Pair {
                            replacement: it.to_string(),
                            display: it.to_string(),
                        })
                        .collect(),
                ));
            }
            Err(UshParseError::UnexpectedFlag(flag, possible_flags)) => {
                let flag_name = flag
                    .value
                    .strip_prefix("--")
                    .expect("flag name should start with --");

                let mut candidates = Vec::new();

                for possible_flag in possible_flags {
                    if possible_flag.starts_with(flag_name) {
                        let f = format!("--{possible_flag}");
                        candidates.push(Pair {
                            replacement: f.clone(),
                            display: f,
                        });
                    }
                }

                return Ok((flag.span.start, candidates));
            }
            Err(UshParseError::MissingFlagGroupRequiredFlag(group, flags)) => {
                if !Self::pos_around_ws(line, pos) {
                    return Ok((0, Vec::with_capacity(0)));
                }

                let mut candidates = Vec::new();

                for flag in flags {
                    let f = format!("--{flag}");
                    candidates.push(Pair {
                        replacement: f.clone(),
                        display: f,
                    });
                }

                return Ok((line.len(), candidates));
            }
            Err(_) => {
                return Ok((0, Vec::with_capacity(0)));
            }
        };

        let mut at = At::new();
        let mut candidates = Vec::new();

        parsed.provide_completions(
            pos,
            &mut at,
            &mut candidates,
            &CompletionContext {
                line,
                cwd: Rc::clone(&self.cwd),
                usermap: Rc::clone(&self.usermap),
            },
        );

        Ok((at.get(), candidates))
    }
}

impl rustyline::highlight::Highlighter for Helper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        let _ = default;
        Cow::Owned(prompt.green().to_string())
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(format!("{}", hint.dimmed().on_bright_black()))
    }

    fn highlight_candidate<'c>(
        &self,
        candidate: &'c str,
        completion: CompletionType,
    ) -> Cow<'c, str> {
        let _ = completion;
        Cow::Owned(candidate.blue().to_string())
    }
}

pub struct Hint {
    display: String,
    completion: Option<String>,
}

impl rustyline::hint::Hint for Hint {
    fn display(&self) -> &str {
        &self.display
    }

    fn completion(&self) -> Option<&str> {
        self.completion.as_deref()
    }
}

impl rustyline::hint::Hinter for Helper {
    type Hint = Hint;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<Self::Hint> {
        let result = parse_line(line);

        match result {
            Ok(_) => None,
            Err(err) => {
                let display = err.to_string();
                let completion =
                    self.hint_completion(err, line.ends_with(char::is_whitespace), line, pos);

                let hint = Hint {
                    display,
                    completion,
                };

                Some(hint)
            }
        }
    }
}

impl Helper {
    fn pos_around_ws(line: &str, pos: usize) -> bool {
        if pos >= line.len() {
            return line.ends_with(char::is_whitespace);
        }

        if pos == 0 {
            return line.starts_with(char::is_whitespace);
        }

        line[pos - 1..pos + 1].chars().all(char::is_whitespace)
    }

    fn hint_completion(
        &self,
        err: UshParseError,
        line_ends_with_space: bool,
        line: &str,
        pos: usize,
    ) -> Option<String> {
        if pos < line.len() {
            return None;
        }

        match err {
            UshParseError::UnknownCommand(command) => {
                let completion = iter_single(UshCommand::get_for_prefix(&command.value))?;

                return Some(completion[command.value.len()..].to_string());
            }
            UshParseError::MissingRequiredArg(arg_name) => {
                let space = if line_ends_with_space { "" } else { " " };

                return Some(format!("{space}{arg_name}:"));
            }
            UshParseError::MissingRequiredArgs(args) => {
                let initial_space = if line_ends_with_space { "" } else { " " };

                let a = args
                    .into_iter()
                    .map(|mut it| {
                        it.push(':');
                        it
                    })
                    .collect::<Vec<_>>()
                    .join(" ");

                return Some(format!("{initial_space}{a}"));
            }
            UshParseError::ExpectedArg(name, possible_args) => {
                if name.span.end != pos {
                    return None;
                }

                let line_subslice = &line[name.span.start..pos];

                let selected_candidate = iter_single(
                    possible_args
                        .into_iter()
                        .filter(|it| it.starts_with(line_subslice)),
                )?;

                return Some(selected_candidate[pos - name.span.start..].to_string());
            }
            UshParseError::UnexpectedFlag(flag, possible_values) => {
                if flag.span.end != pos {
                    return None;
                }

                let line_subslice = &line[flag.span.start..pos];

                let selected_candidate =
                    iter_single(possible_values.into_iter().filter_map(|it| {
                        let c = format!("--{it}");
                        c.starts_with(line_subslice).then_some(c)
                    }))?;

                return Some(selected_candidate[pos - flag.span.start..].to_string());
            }
            _ => {}
        }

        None
    }
}

impl rustyline::validate::Validator for Helper {
    fn validate(&self, ctx: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        let result = parse_line(ctx.input());

        match result {
            Ok(_) => Ok(ValidationResult::Valid(None)),
            Err(err) => Ok(ValidationResult::Invalid(Some(err.to_string()))),
        }
    }
}

/// Returns the only element of an iterator, or `None` if there are more than one.
/// This is useful for checking if an iterator has exactly one element.
///
/// # Examples
///
/// ```
/// use upsilon_shell::iter_single;
///
/// assert_eq!(iter_single(vec![1, 2, 3]), None);
/// assert_eq!(iter_single(vec![1, 2]), None);
/// assert_eq!(iter_single(vec![1]), Some(1));
/// assert_eq!(iter_single(vec![]), None::<i32>);
/// ```
pub fn iter_single<I: IntoIterator>(iter: I) -> Option<I::Item> {
    let mut iter = iter.into_iter();

    let first = iter.next()?;
    let second = iter.next();

    if second.is_some() {
        None
    } else {
        Some(first)
    }
}

impl rustyline::Helper for Helper {}

#[derive(Debug)]
pub struct JwtToken(String);

#[derive(Default, Debug)]
pub struct UserMap {
    map: HashMap<Username, Vec<JwtToken>>,
}

impl UserMap {
    pub fn for_each_user<F>(&self, mut f: F)
    where
        F: FnMut(&Username, &[JwtToken]),
    {
        for (username, tokens) in &self.map {
            f(username, tokens);
        }
    }
}

struct ClientCore {
    usermap: Rc<RefCell<UserMap>>,
    gql_endpoint: String,
    inner_client: reqwest::Client,
}

pub struct Client {
    core: Rc<ClientCore>,
}

impl Client {
    pub fn new(gql_endpoint: String) -> Self {
        Self {
            core: Rc::new(ClientCore {
                usermap: Rc::new(RefCell::new(UserMap::default())),
                gql_endpoint,
                inner_client: reqwest::Client::new(),
            }),
        }
    }

    pub fn usermap(&self) -> &Rc<RefCell<UserMap>> {
        &self.core.usermap
    }

    async fn _gql_query<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: HashMap<String, serde_json::Value>,
        token: impl Into<Option<&str>>,
    ) -> GqlQueryResult<T> {
        let mut req = self.core.inner_client.post(&self.core.gql_endpoint);
        if let Some(token) = token.into() {
            req = req.bearer_auth(token);
        }

        #[derive(Deserialize)]
        struct GqlResponse<T> {
            data: T,
        }

        let res = req
            .json(&serde_json::json!({
                "query": query,
                "variables": variables,
            }))
            .send()
            .await?;

        #[cfg(debug_assertions)]
        let data = {
            let t = res.text().await?;

            debug!("GQL response: {t}");

            serde_json::from_str::<GqlResponse<T>>(&t)?.data
        };

        #[cfg(not(debug_assertions))]
        let data = res.json::<GqlResponse<T>>().await?.data;

        Ok(data)
    }

    pub async fn gql_query<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        token: impl Into<Option<&str>>,
    ) -> GqlQueryResult<T> {
        self.gql_query_with_variables(query, HashMap::new(), token)
            .await
    }

    pub async fn gql_query_with_variables<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: HashMap<String, serde_json::Value>,
        token: impl Into<Option<&str>>,
    ) -> GqlQueryResult<T> {
        self._gql_query(query, variables, token).await
    }

    pub async fn login(&self, username: &Username, password: &str) -> GqlQueryResult<()> {
        #[derive(serde::Deserialize)]
        struct LoginResponse {
            login: String,
        }

        let res = self
            .gql_query_with_variables::<LoginResponse>(
                // language=graphql
                r#"
                mutation Login($username: String!, $password: PlainPassword!) {
                    login(usernameOrEmail: $username, password: $password)
                }            
            "#,
                HashMap::from([
                    ("username".to_string(), serde_json::json!(&username.0)),
                    ("password".to_string(), serde_json::json!(password)),
                ]),
                None,
            )
            .await?;

        let token = JwtToken(res.login);

        self.core
            .usermap
            .borrow_mut()
            .map
            .entry(username.clone())
            .or_default()
            .push(token);

        Ok(())
    }

    pub async fn create_user(
        &self,
        username: &Username,
        password: &str,
        email: &str,
    ) -> GqlQueryResult<()> {
        #[derive(serde::Deserialize)]
        struct CreateUserResponse {
            #[serde(rename = "createUser")]
            create_user: String,
        }

        let res = self
            .gql_query_with_variables::<CreateUserResponse>(
                // language=graphql
                r#"
                mutation CreateUser($username: Username!, $email: Email!, $password: PlainPassword!) {
                    createUser(username: $username, email: $email, password: $password)
                }
            "#,
                HashMap::from([
                    ("username".to_string(), serde_json::json!(&username.0)),
                    ("email".to_string(), serde_json::json!(email)),
                    ("password".to_string(), serde_json::json!(password)),
                ]),
                None,
            )
            .await?;

        let token = JwtToken(res.create_user);

        self.core
            .usermap
            .borrow_mut()
            .map
            .entry(username.clone())
            .or_default()
            .push(token);

        Ok(())
    }
}

pub type GqlQueryResult<T> = Result<T, GqlQueryError>;

#[derive(Debug, thiserror::Error)]
pub enum GqlQueryError {
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests;

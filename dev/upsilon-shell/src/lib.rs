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
use logos::Logos;
use rustyline::completion::{FilenameCompleter, Pair};
use rustyline::error::ReadlineError;
use rustyline::validate::{ValidationContext, ValidationResult};
use rustyline::{ColorMode, CompletionType, Config, Context};

#[derive(Debug, thiserror::Error)]
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
    #[error("expected non-empty arg, got: {0:?}")]
    EmptyArg(Spanned<String>, ArgHint),
    #[error("parse int error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
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

#[derive(Debug, Clone)]
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
);

pub struct CompletionContext<'src> {
    line: &'src str,
    cwd: Rc<RefCell<PathBuf>>,
    usermap: Rc<RefCell<UserMap>>,
}

#[derive(Debug, Clone)]
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

#[derive(Debug)]
pub struct UshCdCommand {
    pub command_name: Spanned<CommandName>,
    pub path: Option<Spanned<UshPath>>,
}

#[derive(Debug)]
pub struct UshLsCommand {
    pub command_name: Spanned<CommandName>,
    pub path: Option<Spanned<UshPath>>,
}

#[derive(Debug)]
pub struct UshPwdCommand {
    pub command_name: Spanned<CommandName>,
}

#[derive(Debug)]
pub struct UshEchoCommand {
    pub command_name: Spanned<CommandName>,
    pub args: Vec<Spanned<String>>,
}

#[derive(Debug)]
pub struct UshExitCommand {
    pub command_name: Spanned<CommandName>,
    pub exit_code: Option<Spanned<i32>>,
}

#[derive(Debug, Clone)]
pub struct Username(pub String);

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

#[derive(Debug)]
pub struct UshLoginCommand {
    pub command_name: Spanned<CommandName>,
    pub username: Spanned<Username>,
    pub password: Spanned<String>,
}

#[derive(Debug)]
pub struct UshCreateUserCommand {
    pub command_name: Spanned<CommandName>,
    pub username: Spanned<Username>,
    pub password: Spanned<String>,
    pub email: Spanned<String>,
}

#[derive(Debug)]
pub struct UshCreateRepoCommand {
    pub command_name: Spanned<CommandName>,
    pub repo_name: Spanned<String>,
}

#[derive(Debug)]
pub struct UshCloneCommand {
    pub command_name: Spanned<CommandName>,
    pub repo_path: Spanned<String>,
    pub arg_to: Option<Spanned<UshPath>>,
    pub to: UshPath,
}

#[derive(Debug)]
pub struct UshHttpUrlCommand {
    pub command_name: Spanned<CommandName>,
    pub repo_path: Spanned<String>,
}

#[derive(Debug)]
pub struct UshGitUrlCommand {
    pub command_name: Spanned<CommandName>,
    pub repo_path: Spanned<String>,
}

#[derive(Debug)]
pub struct UshSshUrlCommand {
    pub command_name: Spanned<CommandName>,
    pub repo_path: Spanned<String>,
}

#[derive(Debug)]
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

#[derive(logos::Logos, Debug, PartialEq, Eq)]
pub enum Token {
    #[regex(r"[~a-zA-Z0-9_\.\\\\/_:\-]+")]
    Value,

    #[regex(r"'.*'")]
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
    fn new(line: &'src str) -> UshParseResult<Self> {
        let mut lexer = UshCommandLexer::new(line);

        let Some((token, span, slice)) = lexer.next() else {return Err(UshParseError::Empty)};
        let Token::Value = token else {return Err(UshParseError::UnexpectedToken(Spanned::new(span, slice.to_string())));};

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
            if let Token::Error = token {
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

    fn parse_clone(&mut self) -> UshParseResult<UshCloneCommand> {
        const REPO_PATH: &str = "remote-path";
        const TO: &str = "to";

        let arg_store = ArgDeclList::new()
            .arg(REPO_PATH, |decl| decl)
            .arg(TO, |decl| decl.optional())
            .parse_from(self)?;

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

#[derive(Default)]
pub struct ArgDeclList {
    decls: Vec<ArgDecl>,
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
        self.decls.push(decl);
        self
    }

    fn names(&self) -> Vec<String> {
        self.decls
            .iter()
            .map(|decl| decl.name.to_string())
            .collect::<Vec<_>>()
    }

    pub fn parse_from(self, parser: &mut UshCommandParser) -> UshParseResult<ArgStore> {
        let mut args = ArgStore::new();

        for decl in &self.decls {
            if let Some(default) = &decl.default {
                args.args
                    .insert(decl.name.to_string(), ArgValue::Default(default.clone()));
            }
        }

        for (token, span, slice) in &mut parser.lexer {
            match token {
                Token::Error => {
                    return Err(UshParseError::UnexpectedToken(span.spanned_string(slice)));
                }
                Token::Value => {
                    let (name, value) = slice.split_once(':').ok_or_else(|| {
                        UshParseError::ExpectedArg(span.spanned_string(slice), self.names())
                    })?;

                    let value_span_start = span.start + name.len() + 1;
                    let value_span = Span {
                        start: value_span_start,
                        ..span
                    };

                    let decl = self
                        .decls
                        .iter()
                        .find(|decl| decl.name == name)
                        .ok_or_else(|| UshParseError::UnexpectedArg(span.spanned_string(name)))?;

                    if value.is_empty() && !decl.allow_empty {
                        return Err(UshParseError::EmptyArg(
                            span.spanned_string(slice),
                            decl.hint,
                        ));
                    }

                    args.args.insert(
                        name.to_string(),
                        ArgValue::Value(
                            Spanned {
                                span: value_span,
                                value: value.to_string(),
                            },
                            span,
                        ),
                    );
                }
                t => {
                    return Err(UshParseError::UnexpectedToken(span.spanned_string(slice)));
                }
            }
        }

        let missing_required_args = self
            .decls
            .iter()
            .filter(|decl| decl.required && !args.args.contains_key(decl.name.as_ref()))
            .map(|decl| decl.name.to_string())
            .collect::<Vec<_>>();

        if !missing_required_args.is_empty() {
            return Err(UshParseError::MissingRequiredArgs(missing_required_args));
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

#[derive(Default)]
pub struct ArgStore {
    args: HashMap<String, ArgValue>,
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

                // at end of line
                return if (pos >= line.len() && line.chars().last().unwrap().is_whitespace())
                    // or somewhere in the middle, but has whitespace on both sides
                    || (pos < line.len()
                    && line[pos - 1..=pos + 1].chars().all(char::is_whitespace))
                {
                    Ok((
                        pos,
                        args.into_iter()
                            .map(|arg| Pair {
                                replacement: format!("{arg}:"),
                                display: arg,
                            })
                            .collect(),
                    ))
                } else {
                    Ok((0, Vec::with_capacity(0)))
                };
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
                cwd: self.cwd.clone(),
                usermap: self.usermap.clone(),
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
                let mut candidates = UshCommand::get_for_prefix(&command.value);

                if candidates.is_empty() || candidates.len() > 1 {
                    return None;
                }

                let completion = candidates.pop().unwrap();

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

                let mut candidates = possible_args
                    .into_iter()
                    .filter(|it| it.starts_with(line_subslice))
                    .collect::<Vec<_>>();

                if candidates.len() != 1 {
                    return None;
                }

                let selected_candidate = candidates.pop().unwrap();

                return Some(selected_candidate[pos - name.span.start..].to_string());
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

impl rustyline::Helper for Helper {}

#[derive(Debug)]
pub struct JwtToken(String);

#[derive(Default, Debug)]
pub struct UserMap {
    map: HashMap<Username, Vec<JwtToken>>,
}

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

use crate::hook_event::HookEvent;

pub mod hook_event;

pub struct HookRegistrar {
    pub(crate) hooks: Vec<Hook>,
}

#[linkme::distributed_slice]
pub static HOOKS: [fn() -> Hook] = [..];

#[derive(Copy, Clone)]
pub struct Hook {
    pub name: &'static str,

    pub(crate) hook_impl: HookImpl,
}

impl Hook {
    pub const fn new(name: &'static str, hook_impl: HookImpl) -> Self {
        Self { name, hook_impl }
    }
}

pub type HookImpl = for<'a> fn(&HookContext<'a>, &HookEvent<'a>) -> HookResult<()>;

#[derive(Copy, Clone)]
pub struct HookContext<'a> {
    pub db: &'a upsilon_data::DataClientMasterHolder,
}

pub type HookResult<T> = Result<T, HookError>;

#[derive(Debug, thiserror::Error)]
pub enum HookError {
    #[error("This hook doesn't handle that event kind")]
    DoesNotHandleEventKind,

    #[error("Hook rejected event")]
    HookRejectedEvent,

    #[error("Hook error from impl: {0}")]
    HookError(#[from] HookImplError),
}

pub type HookImplError = Box<dyn std::error::Error>;

impl HookRegistrar {
    pub fn create() -> Self {
        let hooks = HOOKS.iter().map(|it: &fn() -> Hook| it()).collect();

        Self { hooks }
    }

    pub fn register(&mut self, hook: Hook) {
        self.hooks.push(hook);
    }

    pub fn fire_event(&self, cx: &HookContext, event: &HookEvent) -> HookResult<()> {
        for hook in &self.hooks {
            match (hook.hook_impl)(cx, event) {
                Ok(()) => {}
                Err(HookError::DoesNotHandleEventKind) => continue,
                Err(err) => return Err(err),
            }
        }

        Ok(())
    }
}

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

use std::path::PathBuf;

use crate::{cmd_call, XtaskResult};

pub struct Mdbook {
    pub root: PathBuf,
}

impl Mdbook {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn build(&self) -> XtaskResult<()> {
        cmd_call!(
            "mdbook",
            "build",
            @workdir = &self.root,
        )?;

        Ok(())
    }

    pub fn serve(&self) -> XtaskResult<()> {
        cmd_call!(
            "mdbook",
            "serve",
            @workdir = &self.root,
        )?;

        Ok(())
    }
}

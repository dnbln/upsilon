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

mod app;

use clap::Parser;

use crate::app::*;

#[derive(Debug)]
pub struct ShaShaRef {
    pub old_sha: String,
    pub new_sha: String,
    pub ref_name: String,
}

#[derive(Debug)]
pub struct ShaShaRefLines {
    lines: Vec<ShaShaRef>,
}

impl ShaShaRefLines {
    pub fn iter(&self) -> impl Iterator<Item = &ShaShaRef> {
        self.lines.iter()
    }
}

impl Default for ShaShaRefLines {
    fn default() -> Self {
        let mut lines = vec![];
        for line in std::io::stdin().lines() {
            let line = line.expect("Failed to read line from stdin");

            if line.is_empty() {
                break;
            }

            let mut split = line.splitn(3, ' ');
            let old_sha = split.next().unwrap();
            let new_sha = split.next().unwrap();
            let ref_name = split.next().unwrap();

            lines.push(ShaShaRef {
                old_sha: old_sha.to_string(),
                new_sha: new_sha.to_string(),
                ref_name: ref_name.to_string(),
            });
        }

        Self { lines }
    }
}

type GitHookResult<T> = anyhow::Result<T>;

fn main() -> GitHookResult<()> {
    let app = App::parse();

    match app {
        App::PreReceive(PreReceive { lines }) => {
            println!("pre-receive");

            for line in lines.iter() {
                println!(
                    "pre-receive: {} {} {}",
                    line.old_sha, line.new_sha, line.ref_name
                );
            }
        }
        App::Update(Update {
            ref_name,
            old_oid,
            new_oid,
        }) => {
            println!("update {} {} {}", ref_name, old_oid, new_oid);
        }
        App::PostReceive(PostReceive { lines }) => {
            println!("post-receive");

            for line in lines.iter() {
                println!(
                    "post-receive: {} {} {}",
                    line.old_sha, line.new_sha, line.ref_name
                );
            }
        }
    }

    Ok(())
}

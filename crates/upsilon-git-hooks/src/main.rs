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

use clap::Parser;

#[derive(Parser, Debug)]
enum App {
    #[clap(name = "pre-receive")]
    PreReceive,
    #[clap(name = "update")]
    Update {
        ref_name: String,
        old_oid: String,
        new_oid: String,
    },
    #[clap(name = "post-receive")]
    PostReceive,
}

type GitHookResult<T> = anyhow::Result<T>;

fn main() -> GitHookResult<()> {
    let app = App::parse();

    match app {
        App::PreReceive => {
            println!("pre-receive");
        }
        App::Update {
            ref_name,
            old_oid,
            new_oid,
        } => {
            println!("update {} {} {}", ref_name, old_oid, new_oid);
        }
        App::PostReceive => {
            println!("post-receive");

            for line in std::io::stdin().lines() {
                let line = line?;

                if line.is_empty() {
                    break;
                }

                let mut split = line.splitn(3, ' ');
                let old_oid = split.next().unwrap();
                let new_oid = split.next().unwrap();
                let ref_name = split.next().unwrap();

                println!("post-receive {} {} {}", old_oid, new_oid, ref_name);
            }
        }
    }

    Ok(())
}

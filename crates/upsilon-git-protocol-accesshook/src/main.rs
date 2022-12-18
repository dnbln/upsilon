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

use std::path::PathBuf;

use clap::Parser;

#[derive(clap::ValueEnum, Copy, Clone, Debug)]
#[clap(rename_all = "kebab-case")]
enum GitServiceName {
    UploadPack,
    UploadArchive,
    ReceivePack,
}

#[derive(Parser, Debug)]
struct Command {
    #[arg(value_enum)]
    service_name: GitServiceName,
    path: PathBuf,
    hostname: String,
    canonical_hostname: String,
    ip_address: String,
    tcp_port: String,
}

fn main() {
    let command = Command::parse();
    match &command.service_name {
        GitServiceName::UploadPack => {
            eprintln!(
                "Requested upload-pack: {} {} {} {} {}",
                command.path.display(),
                command.hostname,
                command.canonical_hostname,
                command.ip_address,
                command.tcp_port
            );
        }
        GitServiceName::UploadArchive => {
            eprintln!(
                "Requested upload-archive: {} {} {} {} {}",
                command.path.display(),
                command.hostname,
                command.canonical_hostname,
                command.ip_address,
                command.tcp_port
            );
        }
        GitServiceName::ReceivePack => {
            eprintln!(
                "Requested receive-pack: {} {} {} {} {}",
                command.path.display(),
                command.hostname,
                command.canonical_hostname,
                command.ip_address,
                command.tcp_port
            );

            std::process::exit(1)
        }
    }
}

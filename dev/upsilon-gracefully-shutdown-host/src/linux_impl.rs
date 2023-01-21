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

use std::process::Child;

pub fn kill_child(child: &Child) {
    kill_child_by_id(child.id());
}

fn add_children_to_vec(children: &mut Vec<u32>, child: u32) {
    children.push(child);

    let proc = procfs::process::Process::new(child as i32).unwrap();

    proc.tasks().unwrap().for_each(|task| {
        task.unwrap()
            .children()
            .unwrap()
            .into_iter()
            .for_each(|child| {
                add_children_to_vec(children, child);
            });
    });
}

fn kill_child_by_id(id: u32) {
    let mut children_to_terminate: Vec<u32> = vec![];

    add_children_to_vec(&mut children_to_terminate, id);

    for child in children_to_terminate {
        let success = unsafe { libc::kill(child as libc::pid_t, libc::SIGTERM) == 0 };

        if !success {
            let errno_location: *mut libc::c_int = unsafe { libc::__errno_location() };
            let errno = unsafe { *errno_location };

            if errno == libc::ESRCH {
                continue;
            }
        }

        if !success {
            panic!("Failed to kill child process with id {}", child);
        }
    }
}

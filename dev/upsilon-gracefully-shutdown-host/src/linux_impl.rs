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
    let prep = prepare(child);
    kill_child_with_prep_result(child, prep);
}

pub fn kill_child_with_prep_result(_child: &Child, prep_result: PrepResult) {
    for child in prep_result {
        kill_child_with_sigterm(child);
    }
}

pub type PrepResult = Vec<u32>;

pub fn prepare(child: &Child) -> PrepResult {
    let mut children_to_terminate = vec![];

    add_children_to_vec(&mut children_to_terminate, child.id());

    children_to_terminate
}

fn add_children_to_vec(children: &mut Vec<u32>, child: u32) {
    children.push(child);

    let proc = procfs::process::Process::new(i32::try_from(child).unwrap()).unwrap();

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

fn kill_child_with_sigterm(id: u32) {
    let pid = libc::pid_t::try_from(id).unwrap();

    // SAFETY: correct usage of libc::kill
    #[allow(unsafe_code)]
    let success = unsafe { libc::kill(pid, libc::SIGTERM) == 0 };

    if !success {
        // get errno if failed
        // SAFETY: we are reading errno, can't go wrong.
        #[allow(unsafe_code)]
        let errno: libc::c_int = unsafe { *libc::__errno_location() };

        if errno == libc::ESRCH {
            return;
        }
    }

    if !success {
        panic!("Failed to kill child process with id {id}");
    }
}

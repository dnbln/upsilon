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

pub fn kill_child(_child: &Child) {
    // SAFETY: correct usage of GenerateConsoleCtrlEvent
    #[allow(unsafe_code)]
    let success = unsafe {
        winapi::um::wincon::GenerateConsoleCtrlEvent(
            winapi::um::wincon::CTRL_BREAK_EVENT,
            std::process::id(),
        ) != 0
    };

    if !success {
        panic!("Failed to generate Ctrl+C event");
    }
}

pub type PrepResult = ();

pub fn kill_child_with_prep_result(child: &Child, _prep_result: PrepResult) {
    kill_child(child);
}

pub fn prepare(_child: &Child) -> PrepResult {}

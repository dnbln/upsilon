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

use crate::compile::Compiler;
use crate::Successful;

pub struct Config<C: Compiler + Default> {
    input_to_output: Vec<InputToOutput>,
    fail_fast: bool,
    emit_diagnostics: bool,
    compiler: C,
}

impl<C: Compiler + Default> Default for Config<C> {
    fn default() -> Self {
        Self {
            input_to_output: Vec::new(),
            fail_fast: true,
            emit_diagnostics: true,
            compiler: C::default(),
        }
    }
}

impl<C: Compiler + Default> Config<C> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_file(mut self, input_path: PathBuf, output_path: PathBuf) -> Self {
        self.input_to_output.push(InputToOutput {
            input_path,
            output_path,
        });

        self
    }

    pub fn fail_fast(mut self, fail_fast: bool) -> Self {
        self.fail_fast = fail_fast;

        self
    }

    pub fn emit_diagnostics(mut self, emit_diagnostics: bool) -> Self {
        self.emit_diagnostics = emit_diagnostics;

        self
    }

    pub fn run(self) -> Successful {
        let mut result = Successful::Yes;

        for input_to_output in self.input_to_output.iter() {
            let res = std::fs::read_to_string(&input_to_output.input_path);

            let s = match res {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to read file: {}", e);

                    result = Successful::No;

                    if self.fail_fast {
                        return result;
                    } else {
                        continue;
                    }
                }
            };

            let (parsed, diagnostics) = match crate::parse(Some(PathBuf::from("aaa.modelspec")), &s)
            {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Failed to parse file: {}", e);

                    result = Successful::No;

                    if self.fail_fast {
                        return result;
                    } else {
                        continue;
                    }
                }
            };
            let successful = crate::compile(
                parsed,
                &diagnostics,
                &self.compiler,
                &input_to_output.output_path,
            );

            if !*successful {
                result = Successful::No;

                if self.fail_fast {
                    return result;
                }
            }
        }

        result
    }
}

struct InputToOutput {
    input_path: PathBuf,
    output_path: PathBuf,
}

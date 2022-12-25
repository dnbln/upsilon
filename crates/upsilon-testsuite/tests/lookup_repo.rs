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

use upsilon_test_support::prelude::*;

#[upsilon_test]
#[offline]
async fn simple(cx: &mut TestCx) -> TestResult {
    let id = make_global_mirror_from_host_repo(cx).await?;

    let result = cx.lookup("upsilon").await?;

    assert_eq!(id, result);

    let second_result = cx.lookup("upsilon").await?;

    assert_eq!(id, second_result);

    Ok(())
}

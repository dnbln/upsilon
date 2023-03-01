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

/// <references types="houdini-svelte">

const strNewtype = {"type": "string"};

/** @type {import('houdini').ConfigFile} */
const config = {
    "watchSchema": {
        "url": "http://127.0.0.1:8000/graphql"
    },
    "plugins": {
        "houdini-svelte": {
            "static": true,
        }
    },
    "scalars": {
        "OrganizationId": strNewtype,
        "OrganizationName": strNewtype,
        "OrganizationDisplayName": strNewtype,
        "TeamId": strNewtype,
        "TeamName": strNewtype,
        "TeamDisplayName": strNewtype,
        "RepoId": strNewtype,
        "RepoName": strNewtype,
        "UserId": strNewtype,
        "Username": strNewtype,
        "UserDisplayName": strNewtype,
    }
}

export default config

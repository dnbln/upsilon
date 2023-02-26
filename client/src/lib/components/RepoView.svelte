<!--
  -        Copyright (c) 2023 Dinu Blanovschi
  -
  -    Licensed under the Apache License, Version 2.0 (the "License");
  -    you may not use this file except in compliance with the License.
  -    You may obtain a copy of the License at
  -
  -        https://www.apache.org/licenses/LICENSE-2.0
  -
  -    Unless required by applicable law or agreed to in writing, software
  -    distributed under the License is distributed on an "AS IS" BASIS,
  -    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
  -    See the License for the specific language governing permissions and
  -    limitations under the License.
  -->

<script lang="ts">
    export let repo;

    let repo_path = '/' + repo.path;
    let commit = repo.git.branch.commit;
    let author = commit.author;
    let authorUser = author.user;
    let committer = commit.committer;
    let committerUser = committer.user;
</script>

<div class="repo-view-main">
    <div class="repo-view-top-controls">
        <a href={repo_path}>
            {repo.name}
        </a>

        <div>
            <p>Commit {commit.sha}</p>
            <p>Author: {author.name} &lt;{author.email}&gt;
                {#if authorUser}
                    <a href={'/' + authorUser.username}>{authorUser.username}</a>
                {/if}
            </p>
            <p>Committer: {committer.name} &lt;{committer.email}&gt;
                {#if committerUser}
                    <a href={'/' + committerUser.username}>{committerUser.username}</a>
                {/if}
            </p>
        </div>
    </div>
</div>

<style>
    .repo-view-main {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        width: 100%;
        height: calc(100% - 50px);
    }
</style>
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

<script>
    import Label from "$lib/blobs/Label.svelte";

    export let issue;
</script>

<div class="repo-issue">
    <div class="repo-issue-name">
        <span class="">{issue.name}</span>
    </div>
    <div class="repo-issue-labels">
        {#each issue.labels as label}
            <div class="repo-issue-labels-el">
                <Label {label} />
            </div>
        {/each}
    </div>
    <div class="repo-issue-author">
        <span>{issue.author}</span>
    </div>
    <div class="repo-issue-assignee">
        <span>{issue.assignee}</span>
    </div>
</div>

<style lang="scss">
    .repo-issue {
      display: grid;
      grid-template-columns: 1fr 2fr 1fr 1fr;
      grid-template-rows: 1fr;
      color: whitesmoke;
      font-family: "DejaVu Sans", sans-serif;
    }

    .repo-issue-labels {
      justify-self: left;
      align-self: center;
      display: flex;
      gap: 4px;
      flex-flow: row nowrap;
    }

    .repo-issue-name {
      padding: 20px 10px;
    }

    .repo-issue-author {
      align-self: center;
    }

    .repo-issue-assignee {
      align-self: center;
    }
</style>
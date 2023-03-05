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
    import RepoIssue from "$lib/reusable/RepoIssue.svelte";
    import RepoIssuesControls from "$lib/reusable/RepoIssuesControls.svelte";

    let issues = [
        {id: 1, name: 'Unoptimized loading', author: 'Dinu', labels: [{title: 'new', color: 'hsl(186,68%,22%)'},{title: 'important', color: 'hsl(301,57%,42%)'}], date: new Date('December 17, 2020 03:24:00'), status: {message: 'OPEN', date: ''}},
        {id: 2, name: 'Bad design', author: 'Ecstasy', labels: [{title: 'needs attention', color: 'hsl(337,62%,26%)'}], date: new Date('September 1, 2021 03:24:00'), status: {message: 'CLOSED', date: new Date('March 1, 2023 03:23:23')}},
        {id: 3, name: 'Stuff', author: 'Ecstasy', labels: [{title: 'bad', color: 'hsl(20,77%,24%)'}], date: new Date('December 1, 2022 03:24:00'), status: {message: 'OPEN', date: ''}}
    ]
</script>


<RepoIssuesControls />
<div class="repo-view-issues-main">
    <div class="repo-view-issues">
        <div class="repo-view-issues-top">
            <span class="repo-view-issues-top-name">Issue</span>
            <span>Labels</span>
            <span class="repo-view-issues-top-status">Status</span>
        </div>
        {#each issues as issue}
            <div class="repo-view-issues-el">
                <RepoIssue {issue} />
            </div>
        {/each}
    </div>
</div>

<style lang="scss">
    .repo-view-issues-main {
      width: 70%;
      padding: 0;
      border: hsl(180, 1%, 19%) solid 1px;
      font-family: "DejaVu Sans", sans-serif;
    }

    .repo-view-issues-top {
      display: grid;
      grid-template-columns: 1fr 1fr 1fr;
      align-items: center;
      background-color: hsl(240, 5%, 19%);

      span {
        font-size: 1rem;
        color: whitesmoke;
      }
    }

    .repo-view-issues-top-status {
      text-align: right;
      padding: 0 15px;
    }

    .repo-view-issues-top-name {
      padding: 20px 10px;
    }
</style>

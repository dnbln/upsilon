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

    /**
     * A function to generate "Time since" text on the issues
     * @param date the date of the issue
     * @returns {string} the string result
     */
    function timeSince(date) {
        let seconds = Math.floor((new Date() - date) / 1000);
        let interval = seconds / 31536000;

        if (interval > 1) {
            return Math.floor(interval) + (Math.floor(interval) === 1 ? " year" : " years");
        }
        interval = seconds / 2592000;
        if (interval > 1) {
            return Math.floor(interval) + (Math.floor(interval) === 1 ? " month" : " months");
        }
        interval = seconds / 86400;
        if (interval > 1) {
            return Math.floor(interval) + (Math.floor(interval) === 1 ? " day" : " days");
        }
        interval = seconds / 3600;
        if (interval > 1) {
            return Math.floor(interval) + (Math.floor(interval) === 1 ? " hour" : " hours");
        }
        interval = seconds / 60;
        if (interval > 1) {
            return Math.floor(interval) + (Math.floor(interval) === 1 ? " minute" : " minutes");
        }
        return Math.floor(seconds) + (Math.floor(interval) === 1 ? " second" : " seconds");
    }
</script>

<div class="repo-issue">
    <div class="repo-issue-name">
        <span class="repo-issue-name-text">{issue.name}</span>
        <span class="repo-issue-name-info">#{issue.id} • Issued {timeSince(issue.date)} ago by <a href="/">{issue.author}</a> </span>
    </div>
    <div class="repo-issue-labels">
        {#each issue.labels as label}
            <div class="repo-issue-labels-el">
                <Label {label} />
            </div>
        {/each}
    </div>
    <div class="repo-issue-status">
        <span>{issue.status.message}</span>
        {#if issue.status.message === "CLOSED"}
            <span class="repo-issue-status-date">closed {timeSince(issue.status.date)} ago</span>
        {/if}
    </div>
</div>

<style lang="scss">
  .repo-issue-status-date {
    display: block;
    margin-top: 5px;
    color: hsl(0, 0%, 40%);
    font-size: 0.93em;
  }

  .repo-issue-name-info {
    color: hsl(0, 0%, 40%);
    font-size: 0.93em;

    a {
      color: hsl(0, 0%, 66%);
    }
  }

  .repo-issue {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    grid-template-rows: 1fr;
    color: whitesmoke;
    font-family: "DejaVu Sans", sans-serif;
    border-bottom: hsl(0, 0%, 15%) 1px solid;
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
    display: flex;
    flex-flow: column;
    gap: 5px;
  }

  .repo-issue-author {
    align-self: center;
  }

  .repo-issue-status {
    align-self: center;
    text-align: right;
    padding: 0 15px;
  }
</style>
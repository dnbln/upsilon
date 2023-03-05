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

    export let request;

    // TODO: Separate this function in global script file
    /**
     * A function to generate "Time since" text on the requests
     * @param date the date of the request
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


<div class="repo-request">
    <div class="repo-request-name">
        <div class="repo-request-name-left">
            <img src="/merged.png" alt="">
        </div>
        <div class="repo-request-name-right">
            <span class="repo-request-name-text">{request.name}</span>
            <span class="repo-request-name-info">!{request.id} • requested {timeSince(request.date)} ago by <a href="/">{request.author}</a> </span>
            <span class="repo-request-name-branches"> {request.to} <i class="fa fa-angle-left" style="font-size: 0.8rem"></i> {request.from} </span>
        </div>
    </div>
    <div class="repo-request-labels">
        {#each request.labels as label}
            <div class="repo-request-labels-el">
                <Label {label} />
            </div>
        {/each}
    </div>
    <div class="repo-request-status">
        <span>{request.status.message}</span>
        <span class="repo-request-status-date">last updated {timeSince(request.status.date)} ago</span>
    </div>
</div>

<style lang="scss">
  .repo-request-name-branches {
    font-size: 0.87em;
    color: hsl(0, 0%, 45%);
  }

  .repo-request-status-date {
    display: block;
    margin-top: 5px;
    color: hsl(0, 0%, 40%);
    font-size: 0.93em;
  }

  .repo-request-name-info {
    color: hsl(0, 0%, 60%);
    font-size: 0.93em;

    a {
      color: hsl(0, 0%, 66%);
    }
  }

  .repo-request-name-left {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
  }

  .repo-request {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    grid-template-rows: 1fr;
    color: whitesmoke;
    font-family: "DejaVu Sans", sans-serif;
    border-bottom: hsl(0, 0%, 15%) 1px solid;
  }

  .repo-request-labels {
    justify-self: left;
    align-self: center;
    display: flex;
    gap: 4px;
    flex-flow: row nowrap;
  }

  .repo-request-name {
    padding: 20px 10px;
    display: flex;
    gap: 5px;
  }

  .repo-request-name-right {
    display: flex;
    flex-flow: column;
  }

  .repo-request-author {
    align-self: center;
  }

  .repo-request-status {
    align-self: center;
    text-align: right;
    padding: 0 15px;
  }
</style>
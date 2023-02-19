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
    import { fragment, graphql } from '$houdini';

    export let viewer: import('$houdini').NavBar_viewer | null = null;

    $: navbarViewerInfo = fragment(viewer, graphql`
    fragment NavBar_viewer on User {
        id
        username
    }
    `)
</script>

<nav class="nav-bar">
    <div class="nav-bar-left">
        <div class="nav-bar-item">
            <a href="/"><img src="/upsilon-transparent-white.png" alt="Home"></a>
        </div>
        <div class="nav-bar-item">
            <a href="/docs/book/index.html" rel="external">Docs</a>
        </div>
    </div>

    <div class="nav-bar-right">
        {#if viewer}
        <div class="nav-bar-item">
            <a href="/{viewer.username}">User profile</a>
        </div>
        {/if}
    </div>
</nav>

<style>
    .nav-bar {
        background-color: #333;
        color: white;
        height: 50px;
        overflow: hidden;
        display: flex;
        flex-direction: row;
        justify-content: space-between;
    }

    .nav-bar-left {
        display: flex;
        flex-direction: row;
        justify-content: center;
        align-items: center;
    }

    .nav-bar-right {
        display: flex;
        flex-direction: row;
        justify-content: center;
        align-items: center;
    }

    .nav-bar-item {
        padding: 0 10px;
    }

    .nav-bar a {
        color: white;
        text-decoration: none;
    }

    .nav-bar a:visited {
        color: white;
        text-decoration: none;
    }

    .nav-bar img {
        height: 30px;
        scale: 1;
    }
</style>
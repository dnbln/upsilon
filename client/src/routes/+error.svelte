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
    import { page } from '$app/stores';
</script>

<div class="error-container">
    <h1 class="error-status">{$page.status}</h1>
    <span class="error-status-slash">/</span>
    <p class="error-message">{$page.error.message}</p>
</div>

<style>
    .error-container {
        display: flex;
        flex-direction: row;
        align-items: center;
        justify-content: center;
        min-height: 100vh;
        width: 100%;
        margin: 0;
    }

    .error-status {
        padding: 0 10px;
        color: orange;
    }

    .error-status-slash {
        user-select: none;
        -moz-user-select: none;
        -ms-user-select: none;
        -webkit-user-select: none;
        color: black;
        font-size: 4rem;
    }

    .error-message {
        padding: 0 15px;
        font-size: 2em;
    }
</style>
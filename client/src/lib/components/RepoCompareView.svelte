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
    import RepoTopControls from '$lib/reusable/RepoTopControls.svelte';

    export let repo;
    export let diff;
</script>

<RepoTopControls {repo}/>

<div>
    <p>Files changed: {diff.stats.filesChanged}</p>
    <p>Insertions: {diff.stats.insertions}</p>
    <p>Deletions: {diff.stats.deletions}</p>
</div>

{#each diff.files as {oldPath, newPath, hunks} }
    <p>
        {#if oldPath == null}
            Created file: {newPath}
        {:else if newPath == null}
            Deleted file: {oldPath}
        {:else}
            {#if oldPath !== newPath}
                Renamed {oldPath} -> {newPath}
            {:else}
                Modified {oldPath}
            {/if}
        {/if}
    </p>

    {#each hunks as {oldStart, oldLines, newStart, newLines, lines} }
        <p>
            {oldStart} {oldLines} {newStart} {newLines}
        </p>

        {#each lines as {content, lineType} }
            <pre><code>{lineType}{content}</code></pre>
        {/each}
    {/each}
{/each}
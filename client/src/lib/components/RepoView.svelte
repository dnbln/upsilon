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
	import RepoFileStructure from '$lib/reusable/RepoFileStructure.svelte';
	import RepoFileView from '$lib/components/RepoFileView.svelte';

	export let repo;
	export let commit;
	export let tree;
	export let currentRev = commit.sha;
	export let dirPath: string | undefined;
	export let filePath: string | undefined;
	export let fileContents: string | undefined;

	console.assert(
		!!dirPath ^ (!!filePath && !!fileContents),
		'Either dirPath or (filePath and fileContents) must be defined'
	);

	import RepoTopControls from '../reusable/RepoTopControls.svelte';

	let activeTab = "repo";
</script>

<svelte:head>
	<title>Upsilon | {repo.name}</title>
	<link
		rel="stylesheet"
		href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/4.7.0/css/font-awesome.min.css"
	/>
</svelte:head>

<div class="repo-view-main">
	<RepoTopControls {repo} />

	<div class="repo-navigation">
		<div class="repo-navigation-elements">
			<button on:click={() => activeTab = "repo"} class="repo-navigation-element"><i class="fa fa-terminal repo-navigation-icon"></i>Code</button>
			<button on:click={() => activeTab = "issues"} class="repo-navigation-element"><i class="fa fa-check-circle-o repo-navigation-icon"></i>Issues</button>
			<button on:click={() => activeTab = "merge"} class="repo-navigation-element"><i class="fa fa-random repo-navigation-icon"></i>Merge Requests</button>
			<button on:click={() => activeTab = "wiki"} class="repo-navigation-element"><i class="fa fa-book repo-navigation-icon"></i>Wiki</button>
			<button on:click={() => activeTab = "disc"} class="repo-navigation-element"><i class="fa fa-comments repo-navigation-icon"></i>Discussion</button>
			<button on:click={() => activeTab = "set"} class="repo-navigation-element"><i class="fa fa-gear repo-navigation-icon"></i>Settings</button>
		</div>
		<hr />
	</div>

	{#if activeTab === "repo"}
		{#if dirPath}
			<RepoFileStructure {repo} {currentRev} {tree} {dirPath} />
		{:else if filePath}
			<RepoFileView {repo} {tree} {filePath} {fileContents} />
		{/if}
	{/if}

	{#if activeTab === "issues"}
		<h1>skill issue</h1>
	{/if}

	{#if activeTab === "merge"}
		<h1>merge issue</h1>
	{/if}

	{#if activeTab === "wiki"}
		<h1>wiki issue</h1>
	{/if}

	{#if activeTab === "disc"}
		<h1>disc issue</h1>
	{/if}

	{#if activeTab === "set"}
		<h1>set issue</h1>
	{/if}
</div>

<style lang="scss">
	.repo-view-main {
		width: 100%;
		display: flex;
		flex-flow: column nowrap;
		align-items: center;
		background-color: #1f1f22;
		min-height: calc(100vh - 50px);
	}

	.repo-navigation-icon {
		margin-right: 10px;
	}

	.repo-navigation {
		width: 70%;
		margin-bottom: 10px;
	}

	.repo-navigation-element {
		padding: 10px 20px;
		background-color: hsla(180, 1%, 25%, 20%);
		border-radius: 0.4em;
		border: none;
		color: whitesmoke;
		font-size: 1.05em;

		&:hover {
			background-color: hsl(180, 1%, 25%);
			cursor: pointer;
		}
	}
</style>

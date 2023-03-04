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
	import {HighlightAuto, LineNumbers} from "svelte-highlight";
	import 'highlight.js/styles/tokyo-night-dark.css';
	import RepoVersionControls from "$lib/reusable/RepoVersionControls.svelte";


	export let repo;
	export let tree;
	export let filePath;
	export let fileContents;

	let branches = ['main', 'testing', 'wdadaw'];
	let activeBranch = branches[0];


	function shortenPath(path) {
		if (path.length > 40) {
			let patharr = path.split('/');
			let file = patharr.pop();
			return patharr[0] + '/.../' + file;
		} else {
			return path;
		}
	}

	let showContents = false;
</script>

<div class="repo-file-view">
	<RepoVersionControls {activeBranch} {branches}/>
	<div class="repo-file-view-content">
		<h2>{shortenPath(filePath)}</h2>
		{#if fileContents.length < 3000}
			<div class="repo-file-view-content-code">
				<HighlightAuto code={fileContents} let:highlighted>
					<LineNumbers {highlighted} />
				</HighlightAuto>
			</div>
		{:else}
			{#if showContents}
				<div class="repo-file-view-content-code">
					<HighlightAuto code={fileContents} let:highlighted>
						<LineNumbers {highlighted} />
					</HighlightAuto>
				</div>
			{:else}
				<div class="repo-file-view-warning">
					<p>This file contains a lot of data. Loading it may be heavy for the browser</p>
					<button on:click={() => {showContents = !showContents}}>Show contents</button>
				</div>
			{/if}
		{/if}
	</div>
</div>

<style lang="scss">
	.repo-file-view-warning {
		width: 100%;
		display: flex;
		flex-flow: column;
		align-items: center;
		justify-content: center;
		background-color: #1a1b26;
		padding: 60px 0;

		p {
			font-family: monospace;
		}

		button {
			padding: 10px 20px;
			background-color: hsla(180, 1%, 25%, 20%);
			border-radius: 0.4em;
			border: none;
			color: whitesmoke;
			font-size: 0.95em;

			&:hover {
				background-color: hsl(180, 1%, 25%);
				cursor: pointer;
			}
		}
	}

	.repo-file-view {
		font-family: 'DejaVu Sans', sans-serif;
		width: 100%;
		color: whitesmoke;
		display: flex;
		flex-flow: column nowrap;
		align-items: center;
	}

	.repo-file-view-content {
		width: 70%;
	}

	.repo-file-view-content-code {
		border-radius: 1em;
		background-color: #1a1b26;
		border: hsl(180, 1%, 19%) solid 1px;
		overflow-x: scroll;
	}
</style>

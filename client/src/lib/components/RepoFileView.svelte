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

			// TODO: Improve this algorithm to not delete all folders
			let file = patharr.pop();
			return patharr[0] + '/.../' + file;
		} else {
			return path;
		}
	}
</script>

<div class="repo-file-view">
	<RepoVersionControls {activeBranch} {branches}/>
	<div class="repo-file-view-content">
		<h2>{shortenPath(filePath)}</h2>
		<div class="repo-file-view-content-code">
			<HighlightAuto code={fileContents} let:highlighted>
				<LineNumbers {highlighted} />
			</HighlightAuto>
		</div>
	</div>
</div>

<style lang="scss">
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
		padding: 10px;
		background-color: #1a1b26;
		border: hsl(180, 1%, 19%) solid 1px;
		overflow-x: scroll;
	}
</style>

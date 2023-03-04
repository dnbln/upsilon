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
	import hljs from 'highlight.js';
	import 'highlight.js/styles/tokyo-night-dark.css';


	export let repo;
	export let tree;
	export let filePath;
	export let fileContents;

	/**
	 * A fool-proof function to extract the file type
	 * @param file
	 */
	function getFileType(file) {
		let result = file.trim().toString().toLowerCase().split('.').pop();
		return result ? result : 'Plaintext';
	}

	/**
	 * A function to select the correct class for highlighting
	 * @param file
	 */
	function classType(file) {
		return 'language-' + getFileType(file).toString().toLowerCase();
	}

	let html;

	try {
		html = hljs.highlight(fileContents.toString(), {language: getFileType(filePath)}).value;
	} catch (e) {
		html = fileContents;
	}



</script>

<div class="repo-file-view">
	<h2>{filePath}</h2>
	<div class="repo-file-view-content">
		<pre><code class={classType(filePath)}>{@html html}</code></pre>
	</div>
</div>

<style lang="scss">
	.repo-file-view {
		margin-top: 50px;
		width: 70%;
		color: whitesmoke;
	}

	.repo-file-view-content {
		border-radius: 1em;
		padding: 10px 40px;
		border: hsl(180, 1%, 19%) solid 1px;
	}
</style>

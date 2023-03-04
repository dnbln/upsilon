<script lang="ts" context="module">
	import { dev } from '$app/environment';
	import * as GitTree from '$lib/core/gitTree';
</script>

<script>
	export let repo;
	export let currentRev;
	/**
	 * @type {{entries: ({name: string})[]}}
	 */
	export let tree;
	/**
	 * @type {string}
	 */
	export let dirPath;
	export let readme;

	let parsedTree;

	$: {
		parsedTree = GitTree.makeGitTree(tree);
	}

	/**
	 * A function to compile the file view
	 * @param tree the file tree of the repo
	 * @param dirPath directory path
	 * @returns The file structure ready to be read
	 */
	const compileView = (tree, dirPath) => {
		let t = GitTree.getGitTreeAtPath(tree, dirPath);

		let compiledEntries = [];

		for (const [name, _subtree] of Object.entries(t.dirs)) {
			compiledEntries.push({
				icon: 'fa fa-folder file-icon',
				name: name,
				kind: 'folder',
				commit: 'Initial commit',
				upload: '3 years ago'
			});
		}

		for (const name of t.files) {
			compiledEntries.push({
				icon: 'fa fa-file-text file-icon',
				name: name,
				kind: 'file',
				commit: 'Initial commit',
				upload: '3 years ago'
			});
		}

		return compiledEntries;
	};

	/**
	 * A reactive variable for the files
	 */
	let files;

	$: {
		files = compileView(parsedTree, dirPath);
	}

	let branches = ['main', 'testing', 'wdadaw'];

	let showCloneDropdown = false;
	let uploadFileDropdown = false;
	let activeBranch = branches[0];

	let fileButton;
	let cloneButton;

	/**
	 * Function to close the modals of Add File and Clone.
	 * @param e event
	 */
	function onWindowClick(e) {
		if (fileButton.contains(e.target) === false) {
			uploadFileDropdown = false;
		}

		if (cloneButton.contains(e.target) === false) {
			showCloneDropdown = false;
		}
	}

	/**
	 * Function to give the link for a specific file
	 * @param file the file for which the link will be returned
	 * @returns the link of the file
	 */
	const linkFor = (file) => {
		let dp = dirPath === '/' ? '' : dirPath.endsWith('/') ? dirPath : dirPath + '/';

		if (file.kind === 'folder') {
			return `/${repo.path}/tree/${currentRev}/${dp}${file.name}`;
		} else {
			return `/${repo.path}/blob/${currentRev}/${dp}${file.name}`;
		}
	};
</script>

<svelte:window on:click={onWindowClick} />

<div class="repo-file-structure">
	<div class="repo-file-structure-controls">
		<div class="repo-file-structure-group-left">
			<div class="repo-file-structure-controls-branches">
				<select bind:value={activeBranch} id="button-branch">
					{#each branches as branch}
						<option class="branches-options" value={branch}>{branch}</option>
					{/each}
				</select>
			</div>
			<div class="repo-file-structure-controls-branches-count">
				<i class="fa fa-code-fork"></i>
				<p>{branches.length} Branches</p>
			</div>
		</div>
		<div class="repo-file-structure-group-right">
			<div bind:this={fileButton} class="repo-file-structure-controls-clone">
				<button on:click={() => (uploadFileDropdown = !uploadFileDropdown)} id="button-add">
					<i class="fa fa-file" style="margin-right: 7px;"></i>
					Add file
					<i class="fa fa-angle-down" style="margin-left: 10px; font-size: 1.1rem"></i>
				</button>
				{#if uploadFileDropdown}
					<div class="clone-dropdown">
						<p>dwadawdw</p>
					</div>
				{/if}
			</div>
			<div bind:this={cloneButton} class="repo-file-structure-controls-clone">
				<button on:click={() => (showCloneDropdown = !showCloneDropdown)} id="button-clone">
					Clone <i class="fa fa-angle-down" style="margin-left: 10px; font-size: 1.1rem"></i>
				</button>
				{#if showCloneDropdown}
					<div class="clone-dropdown">
						<p>afni</p>
					</div>
				{/if}
			</div>
		</div>
	</div>
	<table class="repo-file-structure-block">
		<thead>
			<tr id="files-heading">
				<th class="files-columns files-columns-left">File</th>
				<th class="files-columns files-columns-left">Last commit</th>
				<th class="files-columns files-columns-right" id="files-columns-uploaded">Committed on</th>
			</tr>
		</thead>
		<tbody class="repo-file-structure-block-files">
			{#each files as file}
				<tr class="files-rows">
					<td class="files-rows-el files-name">
						<a class="files-rows-el-link" href={linkFor(file)} data-sveltekit-reload={dev ? '' : 'off'}><i class={file.icon}></i>{file.name}</a>
					</td>
					<td class="files-rows-el files-commit">{file.commit}</td>
					<td class="files-rows-el files-date">{file.upload}</td>
				</tr>
			{/each}
		</tbody>
	</table>
	{#if readme}
		<div class="repo-file-structure-readme">
			<h2>{readme.path}</h2>
			<div class="repo-file-structure-readme-file">
				<pre><code>{readme.content}</code></pre>
			</div>
		</div>
	{/if}
</div>

<style>
	.repo-file-structure-readme {
		margin-top: 50px;
		width: 70%;
		color: whitesmoke;
	}

	.repo-file-structure-readme-file {
		border-radius: 1em;
		padding: 10px 40px;
		border: hsl(180, 1%, 19%) solid 1px;
	}

	.repo-file-structure-controls {
		height: 50px;
		width: 70%;
		display: flex;
		justify-content: space-between;
	}

	.repo-file-structure-group-left {
		display: flex;
		gap: 10px;
	}

	.repo-file-structure-group-right {
		display: flex;
		justify-content: end;
		gap: 10px;
	}

	.repo-file-structure-controls-branches-count {
		padding: 0 15px 10px 15px;
		margin: 0;
		display: flex;
		align-items: center;
		gap: 10px;
		color: hsl(0, 0%, 65%);
	}

	.repo-file-structure-controls-branches-count i {
		color: whitesmoke;
	}

	.repo-file-structure-controls-branches-count p {
		margin: 0;
	}

	.repo-file-structure {
		display: flex;
		flex-flow: column nowrap;
		align-items: center;
		font-family: 'DejaVu Sans', sans-serif;
	}

	.clone-dropdown {
		position: absolute;
		background-color: hsl(180, 1%, 19%);
		color: whitesmoke;
		padding: 10px 30px;
		box-shadow: 0 8px 16px 0 rgba(0, 0, 0, 0.2);
		z-index: 1;
	}

	#button-add {
		padding: 10px 15px;
		border-radius: 0.4em;
		border: hsl(180, 1%, 21%) solid 1px;
		background-color: hsl(180, 1%, 19%);
		color: whitesmoke;
	}

	#button-add:hover {
		border: hsl(180, 1%, 21%) solid 1px;
		background-color: hsl(180, 1%, 19%);
		cursor: pointer;
	}

	#button-clone {
		padding: 10px 15px;
		border-radius: 0.4em;
		border: #3cad6e solid 1px;
		background-color: hsl(147, 48%, 46%);
		color: whitesmoke;
	}

	#button-clone:hover {
		border: hsl(147, 48%, 30%) solid 1px;
		background-color: hsl(147, 48%, 30%);
		cursor: pointer;
	}

	.branches-options {
		cursor: pointer;
	}

	#button-branch {
		padding: 10px 15px;
		border-radius: 0.4em;
		border: hsl(180, 1%, 21%) solid 1px;
		background-color: hsl(180, 1%, 19%);
		color: whitesmoke;
	}

	#button-branch:hover {
		border: hsl(180, 1%, 31%) solid 1px;
		background-color: hsl(180, 1%, 39%);
		cursor: pointer;
	}

	.files-date {
		text-align: end;
	}

	/*This appears as unused. It is indeed used so do not remove it*/
	.file-icon {
		width: 20px;
	}

	.files-rows-el-link {
		color: #ffffff;
		text-decoration: none;
		font-weight: bolder;
	}

	table {
		border: hsla(0, 0%, 36%, 0.1) solid 2px;
		width: 70%;
		table-layout: fixed;
		border-collapse: collapse;
	}

	thead {
		height: 30px;
		width: 100%;
		color: #f1f1f6;
		border: none;
		background-color: hsl(180, 1%, 19%);
	}

	.files-columns {
		width: 33%;
		padding: 17px 20px;
		font-size: 0.95rem;
	}

	.files-columns-left {
		text-align: left;
	}

	.files-rows {
		height: 20px;
		border-bottom: hsla(180, 1%, 19%, 0.98) solid 1px;
	}

	.files-rows:hover {
		background-color: hsl(240, 5%, 15%);
	}

	.files-rows-el {
		padding: 15px 20px;
		color: whitesmoke;
		font-size: 0.95rem;
		font-weight: lighter;
	}

	#files-columns-uploaded {
		text-align: right;
	}
</style>

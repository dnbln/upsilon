<script lang="ts" context="module">
	import { dev } from '$app/environment';
</script>

<script>
	import RepoFileView from "$lib/components/RepoFileView.svelte";
	import RepoVersionControls from "$lib/reusable/RepoVersionControls.svelte";

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

	/**
	 * A function to compile the file view
	 * @param tree the file tree of the repo
	 * @param dirPath directory path
	 * @returns The file structure ready to be read
	 */
	const compileView = (tree, dirPath) => {
		let pathFilterPrefix;
		if (dirPath === '/') {
			pathFilterPrefix = '';
		} else {
			pathFilterPrefix = dirPath.endsWith('/') ? dirPath : dirPath + '/';
		}

		let displayedEntries = tree.entries.filter((entry) => entry.name.startsWith(pathFilterPrefix));

		let compiledEntries = [];

		for (const entry of displayedEntries) {
			let name = entry.name.substring(pathFilterPrefix.length);
			let nameSplit = name.split('/');
			let topName = nameSplit[0];

			let index = compiledEntries.findIndex((entry) => entry.name === topName);

			if (index !== -1) {
				compiledEntries[index].icon = 'fa fa-folder file-icon';
				compiledEntries[index].kind = 'folder';
				continue;
			}

			if (nameSplit.length > 1) {
				compiledEntries.push({
					icon: 'fa fa-folder file-icon',
					name: topName,
					kind: 'folder',
					commit: 'Initial commit',
					upload: '3 years ago'
				});
			} else {
				compiledEntries.push({
					icon: 'fa fa-file-text file-icon',
					name: topName,
					kind: 'file',
					commit: 'Initial commit',
					upload: '3 years ago'
				});
			}
		}

		const partition = (array, condition) => {
			return array.reduce(
				([pass, fail], elem) => {
					return condition(elem) ? [[...pass, elem], fail] : [pass, [...fail, elem]];
				},
				[[], []]
			);
		};

		const [folders, files] = partition(compiledEntries, (entry) => entry.kind === 'folder');

		return [...folders, ...files];
	};

	/**
	 * A reactive variable for the files
	 */
	let files;

	$: {
		files = compileView(tree, dirPath);
	}

	let branches = ['main', 'testing', 'wdadaw'];
	let activeBranch = branches[0];

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

<div class="repo-file-structure">
	<RepoVersionControls {activeBranch} {branches}/>
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
	<div class="repo-file-structure-readme">
	</div>
</div>

<style>
	.repo-file-structure-readme {
		margin-top: 50px;
		width: 70%;
		color: whitesmoke;
	}

	.repo-file-structure {
		display: flex;
		flex-flow: column nowrap;
		align-items: center;
		font-family: 'DejaVu Sans', sans-serif;
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

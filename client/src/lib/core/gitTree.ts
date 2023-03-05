/*
 *        Copyright (c) 2023 Dinu Blanovschi
 *
 *    Licensed under the Apache License, Version 2.0 (the "License");
 *    you may not use this file except in compliance with the License.
 *    You may obtain a copy of the License at
 *
 *        https://www.apache.org/licenses/LICENSE-2.0
 *
 *    Unless required by applicable law or agreed to in writing, software
 *    distributed under the License is distributed on an "AS IS" BASIS,
 *    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *    See the License for the specific language governing permissions and
 *    limitations under the License.
 */

export type GitTree = {
	dirs: {
		[key: string]: GitTree;
	};
	files: string[];
};

const splitPath = (path: string): string[] => {
	return path.split('/');
};

const removeDuplicateFilesAndDirs = (tree: GitTree): GitTree => {
	const result: GitTree = { dirs: {}, files: [] };
	for (const file of tree.files) {
		if (!(file in tree.dirs)) {
			result.files.push(file);
		}
	}
	for (const dir in tree.dirs) {
		if (!result.dirs[dir]) {
			result.dirs[dir] = removeDuplicateFilesAndDirs(tree.dirs[dir]);
		}
	}
	return result;
};

export const makeGitTree = (tree: { entries: [{ name: string }] }): GitTree => {
	const root: GitTree = { dirs: {}, files: [] };

	for (let i = 0; i < tree.entries.length; i++) {
		const path = splitPath(tree.entries[i].name);
		let current = root;
		for (let j = 0; j < path.length; j++) {
			const part = path[j];
			if (j === path.length - 1) {
				current.files.push(part);
			} else {
				if (!current.dirs[part]) {
					current.dirs[part] = { dirs: {}, files: [] };
				}
				current = current.dirs[part];
			}
		}
	}

	return removeDuplicateFilesAndDirs(root);
};

export const getGitTreeAtPath = (tree: GitTree, path: string): GitTree | null => {
	if (path === '' || path === '/') {
		return tree;
	}
	const parts = splitPath(path);
	let current = tree;
	for (const part of parts) {
		if (current.dirs[part]) {
			current = current.dirs[part];
		} else {
			return null;
		}
	}
	return current;
};

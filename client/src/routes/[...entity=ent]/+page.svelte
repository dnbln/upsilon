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
<script lang="ts" context="module">
	import NavBar from '$lib/components/NavBar.svelte';
	import UserView from '$lib/components/UserView.svelte';
	import RepoView from '$lib/components/RepoView.svelte';
	import OrganizationView from '$lib/components/OrganizationView.svelte';
	import TeamView from '$lib/components/TeamView.svelte';
</script>

<script lang="ts">
	export let data: import('./$houdini').PageData;
	$: ({ EntityPage } = data);

	let viewer;
	let user;
	let organization;
	let team;
	let repo;

	$: {
		viewer = $EntityPage.data.viewer;
		user = $EntityPage.data.entity?.entityUser;
		organization = $EntityPage.data.entity?.entityOrganization;
		team = $EntityPage.data.entity?.entityTeam;
		repo = $EntityPage.data.entity?.entityRepo;

		if (!user && !organization && !team && !repo) {
			throw { message: 'Not found', code: 404 };
		}
	}
</script>

<NavBar {viewer} />

{#if user}
	<UserView {user} />
{/if}

{#if repo}
	<RepoView {repo} commit={repo.git.branch.commit} tree={repo.git.branch.commit.tree} dirPath="/" />
{/if}

{#if organization}
	<OrganizationView {organization} />
{/if}

{#if team}
	<TeamView {team} />
{/if}

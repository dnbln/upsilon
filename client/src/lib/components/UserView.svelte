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
	export let user;
</script>

<svelte:head>
	<title>User {user.displayName || user.username}</title>
</svelte:head>

<div class="user-view-container">
	<div class="user-view-container-left">
		{#if user.displayName}
			<h1>{user.displayName}</h1>

			<h3>{user.username}</h3>
		{:else}
			<h1>{user.username}</h1>
		{/if}

		<div class="single-info-container">
			<h4>User ID</h4>
			<p>{user.id}</p>
		</div>
	</div>

	<div class="user-view-container-right">
		<h1>Right</h1>
	</div>
</div>

<style>
	.user-view-container {
		display: flex;
		flex-direction: row;
		align-items: center;
		justify-content: center;
		height: calc(100% - 50px);
		width: 100%;
	}

	.user-view-container-left {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		width: 30%;
	}

	.user-view-container-right {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		width: 70%;
	}

	.single-info-container {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		width: 100%;

		height: 50px;
	}

	.single-info-container h4 {
		font-size: 1.3rem;
	}

	.single-info-container p {
		font-size: 1.2rem;
	}
</style>

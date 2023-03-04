<script>
	export let repo;

	let repo_path = '/' + repo.path;
</script>

<div class="repo-hero">
	<div class="repo-hero-title">
		<a href={repo_path} id="repo-view-top-controls-repository-name">{repo.name}</a>
	</div>
</div>

<style>
	.repo-hero {
		height: 200px;
		width: 100%;
		display: flex;
		justify-content: center;
		align-items: center;
		color: #f1f1f6;
		background: linear-gradient(180deg, hsl(120, 12%, 26%) 0%, hsla(0, 0%, 100%, 0) 100%);
	}

	.repo-hero a {
		font-size: 2rem;
	}

	.repo-hero-title {
		display: flex;
		justify-content: center;
		align-items: center;
	}

	#repo-view-top-controls-repository-name {
		color: hsl(120, 42%, 46%);
		text-decoration: none;
	}
</style>

<script>
    export let activeBranch;
    export let branches;

    let fileButton;
    let cloneButton;
    let showCloneDropdown = false;
    let uploadFileDropdown = false;

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
</script>

<svelte:window on:click={onWindowClick} />

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

<style lang="scss">
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
</style>
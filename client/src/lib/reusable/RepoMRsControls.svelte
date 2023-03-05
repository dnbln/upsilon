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
    let newMergeButton;
    let showAddDropdown = false;

    /**
     * Function to close the modals of Add File and Clone.
     * @param e event
     */
    function onWindowClick(e) {
        if (newMergeButton.contains(e.target) === false) {
            showAddDropdown = false;
        }
    }
</script>

<svelte:window on:click={onWindowClick} />

<div class="repo-mrs-controls">
    <div class="repo-mrs-controls-group-left">
        <div class="repo-mrs-controls-search">
            <input id="controls-search" type="text" placeholder="search for merge requests">
        </div>
    </div>
    <div class="repo-mrs-controls-group-right">
        <div bind:this={newMergeButton} class="repo-mrs-controls-add">
            <button on:click={() => (showAddDropdown = !showAddDropdown)} id="button-add">
                New Request <i class="fa fa-plus" style="margin-left: 5px; font-size: 0.7rem"></i>
            </button>
            {#if showAddDropdown}
                <div class="add-dropdown">
                    <p>afni</p>
                </div>
            {/if}
        </div>
    </div>
</div>

<style lang="scss">
  .repo-mrs-controls-search {
    display: flex;
    justify-content: center;
    gap: 5px;
  }

  #controls-search {
    background-color: hsl(236, 5%, 8%);
    border: gray solid 1px;
    border-radius: 5px;
    height: 30px;
    margin: 0;
    padding: 0 5px;
    color: whitesmoke;
    min-width: 300px;
  }

  .repo-file-structure-controls {
    margin-bottom: 10px;
    width: 70%;
    display: flex;
    justify-content: space-between;
  }

  .repo-file-structure-group-left {
    display: flex;
    gap: 10px;
  }

  .repo-mrs-controls-group-right {
    display: flex;
    justify-content: end;
    gap: 10px;
  }

  .add-dropdown {
    position: absolute;
    background-color: hsl(180, 1%, 19%);
    color: whitesmoke;
    padding: 10px 30px;
    box-shadow: 0 8px 16px 0 rgba(0, 0, 0, 0.2);
    z-index: 1;
  }

  #button-add {
    height: 30px;
    padding: 0 15px;
    border-radius: 0.4em;
    border: #3cad6e solid 1px;
    background-color: hsl(147, 48%, 46%);
    color: whitesmoke;
  }

  #button-add:hover {
    border: hsl(147, 48%, 30%) solid 1px;
    background-color: hsl(147, 48%, 30%);
    cursor: pointer;
  }
</style>

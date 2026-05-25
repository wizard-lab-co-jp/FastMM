<script lang="ts">
  import Editor from '$lib/Editor.svelte';
</script>

<svelte:head>
  <title>FastMM Editor</title>
  <style>
    body {
      margin: 0;
      padding: 0;
      background: #121212;
      color: #ffffff;
      min-height: 100vh;
    }
  </style>
</svelte:head>

<main class="container">
  <Editor />
</main>

<style>
  .container {
    width: 100vw;
    min-height: 100vh;
    display: flex;
    justify-content: center;
    padding-top: 5vh;
  }
</style>

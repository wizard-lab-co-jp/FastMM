<script lang="ts">
  import Editor from '$lib/Editor.svelte';
</script>

<svelte:head>
  <title>FastMM Editor</title>
  <style>
    html, body {
      height: 100%;
      margin: 0;
      padding: 0;
      overflow: hidden;
      background: #121212;
      color: #ffffff;
    }
  </style>
</svelte:head>

<div class="container">
  <Editor />
</div>

<style>
  .container {
    width: 100%;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: #121212;
  }
</style>

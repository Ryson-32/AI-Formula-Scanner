<!-- Overlay专用layout，不包含侧边栏等UI组件 -->
<script lang="ts">
  import '../../app.css';
</script>

<main class="overlay-layout">
  <slot />
</main>

<style>
  .overlay-layout {
    width: 100vw;
    height: 100vh;
    margin: 0;
    padding: 0;
    overflow: hidden;
    background: transparent;
  }

  :global(html), :global(body) {
    margin: 0;
    padding: 0;
    overflow: hidden;
    background: transparent !important;
  }
</style>

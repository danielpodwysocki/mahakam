<script setup lang="ts">
useHead({
  link: [
    {
      rel: 'stylesheet',
      href: 'https://fonts.googleapis.com/css2?family=Cinzel:wght@400;700&display=swap',
    },
  ],
})
</script>

<template>
  <div>
    <NuxtPage />
  </div>
</template>

<style>
*, *::before, *::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

:root {
  --bg:            #1e1410;
  --surface:       #2e1e14;
  --input-bg:      #3e2c1e;
  --border-dim:    #8b6040;
  --border-bright: #b07848;
  --accent:        #cc8850;
  --accent-bright: #e8a840;
  --heading:       #dcbc84;
  --text:          #ecddc4;
  --text-dim:      #9e8468;
  --text-muted:    #5e4838;
  --ready:         #5ab870;
  --ready-bg:      rgba(42, 90, 55, 0.22);
  --danger:        #c84444;
  --danger-bg:     rgba(120, 30, 30, 0.28);
}

body {
  background-color: var(--bg);
  background-image:
    radial-gradient(ellipse at 20% 40%, rgba(160, 70, 20, 0.06) 0%, transparent 55%),
    radial-gradient(ellipse at 80% 15%, rgba(120, 70, 20, 0.04) 0%, transparent 50%);
  color: var(--text);
  font-family: 'Georgia', 'Palatino Linotype', Palatino, serif;
  min-height: 100vh;
  line-height: 1.6;
}

::-webkit-scrollbar { width: 8px; height: 8px; }
::-webkit-scrollbar-track { background: var(--surface); }
::-webkit-scrollbar-thumb { background: var(--border-dim); border-radius: 2px; }
::-webkit-scrollbar-thumb:hover { background: var(--accent); }

@keyframes pulse-dim {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.45; }
}
</style>

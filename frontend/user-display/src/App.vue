<script lang="ts" setup>
import { Button, Drawer } from 'primevue';
import { ref } from 'vue';

const drawerVisible = ref(false);
</script>

<template>
  <Drawer v-model:visible="drawerVisible">
    <div :class="$style.drawer">
      <RouterLink to="/" @click.native="drawerVisible = false">ホーム</RouterLink>
      <RouterLink to="/wait-time" @click.native="drawerVisible = false">現在の待ち時間</RouterLink>
      <a href="https://discord.gg/vnm55Yaccc">Discordで通知する</a>
      <a href="https://lin.ee/g5LYSDp">LINEで通知する</a>
      <div :class="$style.spacer"></div>
      <a href="https://github.com/yadokani389/taiyaq">TaiyaQ</a>
    </div>
  </Drawer>
  <Button :class="$style.button" icon="pi pi-bars" severity="secondary" variant="text" rounded
    @click="drawerVisible = true" />
  <RouterView />
</template>

<style module>
.button {
  position: fixed !important;
  top: 25px;
  left: 25px;
  z-index: 1000;
}

.drawer {
  display: flex;
  flex-direction: column;
  gap: 15px;
  padding: 20px;
  height: 100%;
}

.spacer {
  flex: 1;
}
</style>

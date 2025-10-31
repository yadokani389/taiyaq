<script lang="ts" setup>
import { fetchApiWaitTimes } from '@/scripts/api';
import type { WaitTimesResponse } from '@/types';
import { ProgressSpinner } from 'primevue';
import { onMounted, ref } from 'vue';

const waitTimes = ref<{
  tsubuan: number | null;
  custard: number | null;
  kurikinton: number | null;
}>();

onMounted(async () => {
  waitTimes.value = (await fetchApiWaitTimes()).waitTimes;
});
</script>

<template>
  <div :class="$style.container">
    <h1>現在の待ち時間</h1>
    <div v-if="waitTimes" :class="$style.waitTimesContainer">
      <div :class="[$style.waitTimeCard, $style.tsubuan]">
        <strong>つぶあん</strong>
        <span v-if="waitTimes.tsubuan !== null">{{ waitTimes.tsubuan }} <span :class="$style.min">分</span></span>
        <span v-else>現在提供不可</span>
      </div>
      <div :class="[$style.waitTimeCard, $style.custard]">
        <strong>カスタード</strong>
        <span v-if="waitTimes.custard !== null">{{ waitTimes.custard }} <span :class="$style.min">分</span></span>
        <span v-else>現在提供不可</span>
      </div>
      <div :class="[$style.waitTimeCard, $style.kurikinton]">
        <strong>栗きんとん</strong>
        <span v-if="waitTimes.kurikinton !== null">{{ waitTimes.kurikinton }} <span :class="$style.min">分</span></span>
        <span v-else>現在提供不可</span>
      </div>
    </div>
    <div v-else :class="$style.loadingContainer">
      <ProgressSpinner stroke-width="5" />
    </div>
  </div>
</template>

<style module>
.container {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 20px;
  padding: 20px;
  height: 100dvh;
  background-color: #f4f5f7;
}

.waitTimesContainer {
  display: grid;
  grid-template-rows: 1fr 1fr 1fr;
  width: 100%;
  height: 100%;
  gap: 15px;
}

.loadingContainer {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100%;
}

.waitTimeCard {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
  border-radius: 10px;
  position: relative;
}

.waitTimeCard.tsubuan {
  background-color: #fcd7ca;
}

.waitTimeCard.custard {
  background-color: #f5eb96;
}

.waitTimeCard.kurikinton {
  background-color: #ffe0b2;
}

.waitTimeCard strong {
  position: absolute;
  top: 15px;
  left: 15px;
}

.waitTimeCard span {
  font-weight: bold;
  font-size: 85px;
  vertical-align: middle;
  position: relative;
}

.waitTimeCard .min {
  position: absolute;
  opacity: 0.6;
  font-size: 20px;
  right: -27px;
  bottom: 40px;
}
</style>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { client } from './services/api'
import type { DisplayResponse } from './types/api'

const displayData = ref<DisplayResponse>({ ready: [], cooking: [], waiting: [] })
const isLoading = ref(true)
const error = ref<string | null>(null)

let intervalId: number | null = null

function fetchData() {
  error.value = null
  client
    .fetchDisplayOrders()
    .then((data: DisplayResponse) => {
      displayData.value = data
    })
    .catch((err: Error) => {
      error.value = err instanceof Error ? err.message : 'Failed to fetch data'
    })
    .finally(() => {
      isLoading.value = false
    })
}

// スクロール判定の閾値（必要に応じて調整）
const scrollThreshold = 4

// スクロールが必要かどうか
const readyScrolling = computed(() => displayData.value.ready.length >= scrollThreshold)
const cookingScrolling = computed(() => displayData.value.cooking.length >= scrollThreshold)

// シームレスループ用に配列を複製
const readyTrack = computed(() => {
  const arr = displayData.value.ready
  return readyScrolling.value && arr.length > 0 ? [...arr, ...arr] : arr
})
const cookingTrack = computed(() => {
  const arr = [...(displayData.value.cooking || []), ...(displayData.value.waiting || [])]
  return cookingScrolling.value && arr.length > 0 ? [...arr, ...arr] : arr
})

// スクロール速度（項目数に応じて duration を増やす）
const readyDuration = computed(() => `${Math.max(8, (displayData.value.ready.length || 1) * 1.2)}s`)
const cookingDuration = computed(
  () => `${Math.max(8, (displayData.value.cooking.length || 1) * 1.2)}s`,
)

onMounted(() => {
  fetchData()
  intervalId = setInterval(fetchData, 2500)
})

onUnmounted(() => {
  if (intervalId) {
    clearInterval(intervalId)
  }
})
</script>

<template>
  <div class="display-screen">
    <h1>たい焼き注文状況</h1>

    <div v-if="isLoading" class="loading">読み込み中...</div>

    <div v-else-if="error" class="error">エラー: {{ error }}</div>

    <div v-else class="orders">
      <div class="ready-section">
        <h2>お渡し準備完了</h2>
        <div v-if="displayData.ready.length === 0" class="no-orders">なし</div>
        <div
          v-else
          class="order-numbers-viewport"
          :class="{ scrolling: readyScrolling }"
          :style="{ '--scroll-duration': readyDuration }"
        >
          <div class="scroll-track">
            <span
              v-for="(order, idx) in readyTrack"
              :key="order.id + '-' + idx"
              class="order-number ready"
            >
              {{ order.id }}
            </span>
          </div>
        </div>
      </div>

      <div class="cooking-section">
        <h2>調理中</h2>
        <div v-if="displayData.cooking.length === 0" class="no-orders">なし</div>
        <div
          v-else
          class="order-numbers-viewport"
          :class="{ scrolling: cookingScrolling }"
          :style="{ '--scroll-duration': cookingDuration }"
        >
          <div class="scroll-track">
            <span
              v-for="(order, idx) in cookingTrack"
              :key="order.id + '-' + idx"
              class="order-number cooking"
            >
              {{ order.id }}
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.display-screen {
  max-width: 1200px;
  margin: 0 auto;
  padding: 1.5rem;
  text-align: center;
  font-family: system-ui, sans-serif;
  box-sizing: border-box;
  min-height: 100vh; /* 画面全体に収める */
  display: flex;
  flex-direction: column;
}

h1 {
  font-size: clamp(1.8rem, 3.5vw, 2.8rem);
  margin-bottom: 1.2rem;
  color: #333;
}

h2 {
  font-size: clamp(1.4rem, 2.2vw, 1.8rem);
  margin-bottom: 0.8rem;
  color: #555;
}

.loading,
.error {
  font-size: 1.2rem;
  padding: 2rem;
}

.error {
  color: #e74c3c;
  background-color: #fdf2f2;
  border-radius: 8px;
}

.orders {
  display: flex;
  flex-direction: row;
  gap: 1.5rem;
  align-items: stretch;
  flex: 1 1 auto;
  overflow: hidden; /* 外側で余分なスクロールを作らない */
}

/* 各セクションを横に広げる。min-width:0 で flex 内で正しく縮む */
.ready-section,
.cooking-section {
  flex: 1 1 0;
  min-width: 0;
  padding: 1.5rem;
  border-radius: 12px;
  box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
  display: flex;
  flex-direction: column;
}

/* 背景色は維持 */
.ready-section {
  background-color: #d4edda;
  border: 2px solid #c3e6cb;
}

.cooking-section {
  background-color: #fff3cd;
  border: 2px solid #ffeaa7;
}

/* スクロール用ビューポート */
.order-numbers-viewport {
  width: 100%;
  overflow: hidden;
  position: relative;
  margin-top: 0.5rem;
  /* 画面に収めるための最大高さ（header や余白を考慮） */
  max-height: calc(100vh - 220px);
  /* 縦スクロール用に縦方向の配置を想定 */
  display: block;
}

/* スクロール領域（トラック） */
.scroll-track {
  display: flex;
  flex-direction: column; /* 縦に並べる */
  gap: 0.5rem;
  align-items: stretch;
  will-change: transform;
}

/* 自動スクロール時 */
.order-numbers-viewport.scrolling .scroll-track {
  animation: scroll-up var(--scroll-duration) linear infinite;
}

/* トラックは要素を複製してある前提で半分だけ移動 */
@keyframes scroll-up {
  from {
    transform: translateY(0);
  }
  to {
    transform: translateY(-50%);
  }
}

/* 注文番号を大きく表示。clamp で最大値と最小値を指定 */
.order-number {
  /* 3桁が大きく見えるように行単位で大きめに設定 */
  font-size: clamp(2.4rem, 8vw, 5rem);
  font-weight: bold;
  padding: 0.6rem 1rem;
  border-radius: 8px;
  width: 100%;
  display: block;
  text-align: center;
  box-sizing: border-box;
  flex-shrink: 0;
}

.order-number.ready {
  background-color: #28a745;
  color: white;
}

.order-number.cooking {
  background-color: #ffc107;
  color: #333;
}

.no-orders {
  color: #6c757d;
  font-style: italic;
  font-size: 1.1rem;
}

/* レスポンシブ調整 */
@media (max-width: 900px) {
  .orders {
    flex-direction: column;
    gap: 1rem;
    align-items: stretch;
  }
  .order-number {
    font-size: clamp(1.8rem, 6.5vw, 3rem);
    padding: 0.5rem 0.8rem;
  }
}

@media (max-width: 600px) {
  .display-screen {
    padding: 0.8rem;
  }

  h1 {
    font-size: 1.6rem;
  }

  h2 {
    font-size: 1.2rem;
  }

  .order-number {
    font-size: 1.1rem;
    padding: 0.4rem 0.6rem;
  }
}
</style>

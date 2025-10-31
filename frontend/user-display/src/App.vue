<script setup lang="ts">
import { computed, onMounted, ref, triggerRef } from 'vue';
import { fetchApiOrdersDisplay, fetchApiOrdersId } from './scripts/api';
import { computedAsync } from '@vueuse/core';
import type { OrdersDisplayResponse } from './types';
import { ProgressSpinner, Select } from 'primevue';

const orders = ref<OrdersDisplayResponse>();
const flattenedOrders = computed(() => {
  if (!orders.value) return [];
  return [
    ...orders.value.cooking.map(order => ({ ...order, status: 'cooking' as const })),
    ...orders.value.ready.map(order => ({ ...order, status: 'ready' as const })),
    ...orders.value.waiting.map(order => ({ ...order, status: 'waiting' as const })),
  ].toSorted((a, b) => b.id - a.id);
});

const orderId = ref<number>();
const orderDetail = computedAsync(async () => {
  if (!orderId.value) return undefined;
  return await fetchApiOrdersId(orderId.value!);
}, undefined, { lazy: true });

onMounted(async () => {
  orders.value = await fetchApiOrdersDisplay();
  setInterval(refresh, 5000);
});

const refresh = async () => {
  orders.value = await fetchApiOrdersDisplay();
  triggerRef(orderId);
};
</script>

<template>
  <div v-if="orders" :class="$style.mainContainer">
    <div :class="$style.orderDetail">
      <div :class="$style.selectContainer">
        <span>あなたの注文番号を選択：</span>
        <Select v-model="orderId" size="large" :options="flattenedOrders" :option-value="order => order.id"
          :option-label="order => `#${order.id}`" :class="$style.select" />
      </div>
      <div v-if="orderDetail" :class="$style.orderDetailContainer">
        <div v-if="orderDetail.status === 'waiting'">
          準備中: あと約
          {{ orderDetail.estimatedWaitMinutes }}
          分
        </div>
        <div v-else-if="orderDetail.status === 'cooking'">調理中: まもなく完成します</div>
        <div v-else-if="orderDetail.status === 'ready'">受取待ち</div>
        <div v-else-if="orderDetail.status === 'completed'">受け取り済み</div>
        <div v-else-if="orderDetail.status === 'cancelled'">キャンセル済み</div>

        <div :class="$style.orderedItems">
          <div v-for="item in orderDetail.items" :class="[$style.itemCard, {
            [$style.tsubuan]: item.flavor === 'tsubuan',
            [$style.custard]: item.flavor === 'custard',
            [$style.kurikinton]: item.flavor === 'kurikinton',
          }]">
            <span>
              {{
                {
                  tsubuan: 'つぶあん',
                  custard: 'カスタード',
                  kurikinton: '栗きんとん',
                }[item.flavor]
              }}
            </span>
            <span>x {{ item.quantity }}</span>
          </div>
        </div>
      </div>
      <div v-else-if="!orderId"></div>
      <div v-else :class="$style.orderDetailLoadingContainer">
        <ProgressSpinner stroke-width="5" />
      </div>
    </div>
    <div :class="$style.ordersList">
      <div v-for="order in flattenedOrders" :class="[$style.orderCard, {
        [$style.cooking]: order.status === 'cooking',
        [$style.ready]: order.status === 'ready',
        [$style.waiting]: order.status === 'waiting',
      }]">
        <span>#{{ order.id }}</span>
        <span :class="$style.status">
          {{
            {
              cooking: '調理中',
              ready: '受取待ち',
              waiting: '準備中',
            }[order.status]
          }}
        </span>
      </div>
    </div>
  </div>
  <div v-else :class="$style.loadingContainer">
    <ProgressSpinner stroke-width="5" />
  </div>
</template>

<style module>
.loadingContainer {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100dvh;
}

.mainContainer {
  display: flex;
  flex-direction: column;
  height: 100dvh;
  gap: 10px;
  padding: 10px;
}

.mainContainer>div {
  border-radius: 8px;
}

.orderDetail {
  background-color: #f4f5f7;
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 20px;
  padding: 20px;
  position: relative;
}

.selectContainer {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
}

.select {
  width: 150px;
  font-weight: bold;
}

.select span {
  font-size: 30px !important;
}

.orderDetailContainer {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  font-size: 24px;
}

.orderDetailLoadingContainer {
  display: flex;
  justify-content: center;
  align-items: center;
  flex: 1;
}

.orderedItems {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.itemCard {
  display: flex;
  justify-content: space-between;
  width: 300px;
  font-size: 20px;
  padding: 10px;
  background-color: #ffffff3b;
  border: solid 1px;
  border-radius: 4px;
}

.itemCard.tsubuan {
  background-color: #fff0eb;
  border-color: #ffccbc;
}

.itemCard.custard {
  background-color: #fff9c4;
  border-color: #dfd57c;
}

.itemCard.kurikinton {
  background-color: #ffe0b2;
  border-color: #ffcc80;
}

.ordersList {
  background-color: #f4f5f7;
  padding: 10px;
  flex: 1;
  gap: 10px;
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
}

.orderCard {
  display: flex;
  cursor: pointer;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  height: 100px;
  padding: 10px;
  margin-bottom: 8px;
  border-radius: 4px;
}

.orderCard span {
  font-size: 30px;
  font-weight: bold;
  line-height: 1.2em;
}

.orderCard .status {
  font-size: 17px;
}

.orderCard.cooking .status {
  color: #f57c00;
}

.orderCard.ready .status {
  color: #388e3c;
}

.orderCard.waiting .status {
  color: #d32f2f;
}

.orderCard.cooking {
  background-color: #ffecb3;
}

.orderCard.ready {
  background-color: #c8e6c9;
}

.orderCard.waiting {
  background-color: #ffcdd2;
}
</style>

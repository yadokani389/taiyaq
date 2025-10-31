<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ordersApi, productionApi, flavorsApi, stockApi, apiClient } from './api'
import type { Order, Flavor } from './api/types'

const currentView = ref<'order' | 'baking' | 'settings'>('order')
const orders = ref<Order[]>([])
const stockData = ref<{ つぶあん: number; カスタード: number; 栗きんとん: number }>({
  つぶあん: 0,
  カスタード: 0,
  栗きんとん: 0,
})

const orderForm = ref({
  つぶあん: 0,
  カスタード: 0,
  栗きんとん: 0,
  priority: false,
})

const bakingForm = ref({
  つぶあん: 0,
  カスタード: 0,
  栗きんとん: 0,
})

const settings = ref({
  つぶあん: { cookingTimeMinutes: 30, quantityPerBatch: 10 },
  カスタード: { cookingTimeMinutes: 25, quantityPerBatch: 8 },
  栗きんとん: { cookingTimeMinutes: 35, quantityPerBatch: 6 },
})

const showAppSettings = ref(false)
const appSettings = ref({
  baseUrl: 'http://localhost:3000',
  token: '',
})

const statusFilters = ref({
  all: true,
  waiting: false,
  cooking: false,
  ready: false,
  completed: false,
  cancelled: false,
})

const handleAllFilter = () => {
  if (statusFilters.value.all) {
    statusFilters.value.waiting = false
    statusFilters.value.cooking = false
    statusFilters.value.ready = false
    statusFilters.value.completed = false
    statusFilters.value.cancelled = false
  }
}

const handleStatusFilter = () => {
  statusFilters.value.all = false
}

const filteredOrders = computed(() => {
  if (statusFilters.value.all) {
    return orders.value
  }

  const activeFilters = Object.entries(statusFilters.value)
    .filter(([key, value]) => key !== 'all' && value)
    .map(([key]) => key)

  if (activeFilters.length === 0) {
    return orders.value
  }

  return orders.value.filter((order) => activeFilters.includes(order.status))
})

// 追加: 日本語キーの型と flavor コード -> 日本語キー のマッピング
type FlavorCounts = { つぶあん: number; カスタード: number; 栗きんとん: number }
const flavorCodeToJP: Record<Flavor, keyof FlavorCounts> = {
  tsubuan: 'つぶあん',
  custard: 'カスタード',
  kurikinton: '栗きんとん',
}

const statusCounts = computed(() => {
  // 明確な型を付与
  const counts: { waiting: FlavorCounts; cooking: FlavorCounts; waitingAndCooking: FlavorCounts } =
    {
      waiting: { つぶあん: 0, カスタード: 0, 栗きんとん: 0 },
      cooking: { つぶあん: 0, カスタード: 0, 栗きんとん: 0 },
      waitingAndCooking: { つぶあん: 0, カスタード: 0, 栗きんとん: 0 },
    }

  orders.value.forEach((order) => {
    if (order.status === 'waiting' || order.status === 'cooking') {
      order.items.forEach((item) => {
        const flavorName = flavorCodeToJP[item.flavor] as keyof FlavorCounts
        const statusKey = order.status as 'waiting' | 'cooking'
        counts[statusKey][flavorName] += item.quantity
        counts.waitingAndCooking[flavorName] += item.quantity
      })
    }
  })

  return counts
})

const addOrder = () => {
  currentView.value = 'order'
}

const reportBaking = () => {
  currentView.value = 'baking'
}

const openSettings = async () => {
  currentView.value = 'settings'
  await loadFlavorConfigs()
}

const loadFlavorConfigs = async () => {
  try {
    const response = await flavorsApi.getFlavorConfigs()
    console.log('Loaded flavor configs:', response.data)

    const configs = response.data
    if (configs?.tsubuan) {
      settings.value.つぶあん = configs.tsubuan
    }
    if (configs?.custard) {
      settings.value.カスタード = configs.custard
    }
    if (configs?.kurikinton) {
      settings.value.栗きんとん = configs.kurikinton
    }
  } catch (error) {
    console.error('Failed to load flavor configs:', error)
    alert('設定の読み込みに失敗しました')
  }
}

const submitOrder = async () => {
  const items = []

  if (orderForm.value.つぶあん > 0) {
    items.push({ flavor: 'tsubuan' as Flavor, quantity: orderForm.value.つぶあん })
  }
  if (orderForm.value.カスタード > 0) {
    items.push({ flavor: 'custard' as Flavor, quantity: orderForm.value.カスタード })
  }
  if (orderForm.value.栗きんとん > 0) {
    items.push({ flavor: 'kurikinton' as Flavor, quantity: orderForm.value.栗きんとん })
  }

  if (items.length === 0) {
    alert('少なくとも1つの数量を選択してください')
    return
  }

  // 確認画面
  const itemsText = items
    .map((item) => `${getFlavorName(item.flavor)}: ${item.quantity}個`)
    .join('\n')
  const priorityText = orderForm.value.priority ? '優先注文' : '通常注文'
  const confirmMessage = `以下の注文を追加しますか？\n\n${itemsText}\n\n${priorityText}`

  if (!confirm(confirmMessage)) {
    return
  }

  const requestData = {
    items,
    isPriority: orderForm.value.priority,
  }

  console.log('Creating order:', requestData)
  const response = await ordersApi.createOrder(requestData)
  console.log('Create order response:', response)

  if (response.error) {
    alert(`注文の追加に失敗しました: ${response.error.message}`)
    return
  }

  orderForm.value = {
    つぶあん: 0,
    カスタード: 0,
    栗きんとん: 0,
    priority: false,
  }

  await fetchOrders()
}

const submitBaking = async () => {
  const items = []

  if (bakingForm.value.つぶあん > 0) {
    items.push({ flavor: 'tsubuan' as Flavor, quantity: bakingForm.value.つぶあん })
  }
  if (bakingForm.value.カスタード > 0) {
    items.push({ flavor: 'custard' as Flavor, quantity: bakingForm.value.カスタード })
  }
  if (bakingForm.value.栗きんとん > 0) {
    items.push({ flavor: 'kurikinton' as Flavor, quantity: bakingForm.value.栗きんとん })
  }

  if (items.length === 0) {
    alert('少なくとも1つの味を選択してください')
    return
  }

  // 確認画面
  const itemsText = items
    .map((item) => `${getFlavorName(item.flavor)}: ${item.quantity}個`)
    .join('\n')
  const confirmMessage = `以下の焼き上がりを報告しますか？\n\n${itemsText}`

  if (!confirm(confirmMessage)) {
    return
  }

  const requestData = { items }
  console.log('Reporting production:', requestData)
  const response = await productionApi.reportProduction(requestData)
  console.log('Production report response:', response)

  if (response.error) {
    alert(`焼き上がりの報告に失敗しました: ${response.error.message}`)
    return
  }

  if (response.data?.newlyReadyOrders && response.data.newlyReadyOrders.length > 0) {
    alert(`${response.data.newlyReadyOrders.length}件のオーダーが準備完了になりました`)
  }

  bakingForm.value = {
    つぶあん: 0,
    カスタード: 0,
    栗きんとん: 0,
  }

  await fetchOrders()
}

const cancelOrder = async (orderId: number) => {
  if (confirm('このオーダーをキャンセルしますか？')) {
    console.log('Cancelling order:', orderId)
    const response = await ordersApi.cancelOrder(orderId)
    console.log('Cancel order response:', response)

    if (response.error) {
      alert(`キャンセルに失敗しました: ${response.error.message}`)
      return
    }

    await fetchOrders()
  }
}

const increasePriority = async (orderId: number) => {
  if (confirm('このオーダーの優先度を上げますか？')) {
    const requestData = {
      isPriority: true,
    }
    console.log('Updating priority for order:', orderId, requestData)
    const response = await ordersApi.updatePriority(orderId, requestData)
    console.log('Update priority response:', response)

    if (response.error) {
      alert(`優先度の変更に失敗しました: ${response.error.message}`)
      return
    }

    await fetchOrders()
  }
}

const completeOrder = async (orderId: number) => {
  if (confirm('このオーダーを完了しますか？')) {
    console.log('Completing order:', orderId)
    const response = await ordersApi.completeOrder(orderId)
    console.log('Complete order response:', response)

    if (response.error) {
      alert(`完了処理に失敗しました: ${response.error.message}`)
      return
    }

    await fetchOrders()
  }
}

const saveAppSettings = () => {
  // Update API client (handles localStorage persistence)
  apiClient.setBaseUrl(appSettings.value.baseUrl)
  apiClient.setToken(appSettings.value.token)

  showAppSettings.value = false
}

const loadAppSettings = () => {
  // Get values from ApiClient (single source of truth)
  appSettings.value.baseUrl = apiClient.getBaseUrl()
  appSettings.value.token = apiClient.getToken() || ''
}

const fetchOrders = async () => {
  console.log('Fetching all orders')
  const response = await ordersApi.getOrders()
  console.log('Fetch orders response:', response)

  if (response.error) {
    alert(`オーダーの取得に失敗しました: ${response.error.message}`)
    return
  }

  if (response.data) {
    orders.value = response.data
  }

  await fetchStock()
}

const fetchStock = async () => {
  console.log('Fetching stock data')
  const response = await stockApi.getStock()
  console.log('Fetch stock response:', response)

  if (response.error) {
    console.error(`在庫データの取得に失敗しました: ${response.error.message}`)
    return
  }

  if (response.data) {
    const flavorCodeToJP = {
      tsubuan: 'つぶあん',
      custard: 'カスタード',
      kurikinton: '栗きんとん',
    }

    stockData.value = {
      つぶあん: response.data.tsubuan,
      カスタード: response.data.custard,
      栗きんとん: response.data.kurikinton,
    }
  }
}

const getFlavorName = (flavor: string) => {
  const flavorMap: Record<string, string> = {
    tsubuan: 'つぶあん',
    custard: 'カスタード',
    kurikinton: '栗きんとん',
  }
  return flavorMap[flavor] || flavor
}

const saveFlavorSettings = async () => {
  try {
    console.log('Saving flavor settings:', settings.value)
    const responses = await Promise.all([
      flavorsApi.updateFlavorConfig('tsubuan' as Flavor, settings.value.つぶあん),
      flavorsApi.updateFlavorConfig('custard' as Flavor, settings.value.カスタード),
      flavorsApi.updateFlavorConfig('kurikinton' as Flavor, settings.value.栗きんとん),
    ])
    console.log('Flavor settings responses:', responses)
    alert('設定を保存しました')
  } catch (error) {
    console.error('Failed to save flavor settings:', error)
    alert('設定の保存に失敗しました')
  }
}

onMounted(() => {
  loadAppSettings()
  fetchOrders()
})
</script>

<template>
  <div class="app">
    <div class="left-panel">
      <div class="button-group">
        <button @click="addOrder" :class="{ active: currentView === 'order' }">注文の追加</button>
        <button @click="reportBaking" :class="{ active: currentView === 'baking' }">
          焼き上がりの報告
        </button>
        <button @click="openSettings" :class="{ active: currentView === 'settings' }">設定</button>
      </div>

      <div class="component-area">
        <div v-if="currentView === 'order'" class="order-form">
          <h3>注文の追加</h3>
          <div class="form-group">
            <label>つぶあん:</label>
            <select v-model="orderForm.つぶあん">
              <option v-for="n in 21" :key="n - 1" :value="n - 1">{{ n - 1 }}個</option>
            </select>
          </div>
          <div class="form-group">
            <label>カスタード:</label>
            <select v-model="orderForm.カスタード">
              <option v-for="n in 21" :key="n - 1" :value="n - 1">{{ n - 1 }}個</option>
            </select>
          </div>
          <div class="form-group">
            <label>栗きんとん:</label>
            <select v-model="orderForm.栗きんとん">
              <option v-for="n in 21" :key="n - 1" :value="n - 1">{{ n - 1 }}個</option>
            </select>
          </div>
          <div class="form-group">
            <label>
              <input type="checkbox" v-model="orderForm.priority" />
              優先度
            </label>
          </div>
          <button @click="submitOrder" class="submit-btn">注文を追加</button>
        </div>
        <div v-if="currentView === 'baking'" class="baking-form">
          <h3>焼き上がりの報告</h3>
          <div class="form-group">
            <label>つぶあん:</label>
            <select v-model="bakingForm.つぶあん">
              <option v-for="n in 21" :key="n - 1" :value="n - 1">{{ n - 1 }}個</option>
            </select>
          </div>
          <div class="form-group">
            <label>カスタード:</label>
            <select v-model="bakingForm.カスタード">
              <option v-for="n in 21" :key="n - 1" :value="n - 1">{{ n - 1 }}個</option>
            </select>
          </div>
          <div class="form-group">
            <label>栗きんとん:</label>
            <select v-model="bakingForm.栗きんとん">
              <option v-for="n in 21" :key="n - 1" :value="n - 1">{{ n - 1 }}個</option>
            </select>
          </div>
          <button @click="submitBaking" class="submit-btn">焼き上がりを報告</button>
        </div>
        <div v-if="currentView === 'settings'" class="settings-form">
          <h3>調理時間・バッチサイズ設定</h3>

          <div class="flavor-setting">
            <h4>つぶあん</h4>
            <div class="setting-row">
              <div class="form-group">
                <label>調理時間 (分):</label>
                <input
                  type="number"
                  v-model.number="settings.つぶあん.cookingTimeMinutes"
                  min="1"
                />
              </div>
              <div class="form-group">
                <label>バッチサイズ (個):</label>
                <input type="number" v-model.number="settings.つぶあん.quantityPerBatch" min="1" />
              </div>
            </div>
          </div>

          <div class="flavor-setting">
            <h4>カスタード</h4>
            <div class="setting-row">
              <div class="form-group">
                <label>調理時間 (分):</label>
                <input
                  type="number"
                  v-model.number="settings.カスタード.cookingTimeMinutes"
                  min="1"
                />
              </div>
              <div class="form-group">
                <label>バッチサイズ (個):</label>
                <input
                  type="number"
                  v-model.number="settings.カスタード.quantityPerBatch"
                  min="1"
                />
              </div>
            </div>
          </div>

          <div class="flavor-setting">
            <h4>栗きんとん</h4>
            <div class="setting-row">
              <div class="form-group">
                <label>調理時間 (分):</label>
                <input
                  type="number"
                  v-model.number="settings.栗きんとん.cookingTimeMinutes"
                  min="1"
                />
              </div>
              <div class="form-group">
                <label>バッチサイズ (個):</label>
                <input
                  type="number"
                  v-model.number="settings.栗きんとん.quantityPerBatch"
                  min="1"
                />
              </div>
            </div>
          </div>
          <button @click="saveFlavorSettings" class="submit-btn">設定を保存</button>
        </div>
      </div>
    </div>

    <div class="right-panel">
      <div class="right-header">
        <h2>オーダー一覧</h2>
        <button @click="showAppSettings = true" class="app-settings-btn">アプリ設定</button>
      </div>
      <div class="order-list">
        <div class="filters-row">
          <div class="status-filters">
            <label
              ><input type="checkbox" v-model="statusFilters.all" @change="handleAllFilter" />
              All</label
            >
            <label
              ><input
                type="checkbox"
                v-model="statusFilters.waiting"
                @change="handleStatusFilter"
              />
              waiting</label
            >
            <label
              ><input
                type="checkbox"
                v-model="statusFilters.cooking"
                @change="handleStatusFilter"
              />
              cooking</label
            >
            <label
              ><input type="checkbox" v-model="statusFilters.ready" @change="handleStatusFilter" />
              ready</label
            >
            <label
              ><input
                type="checkbox"
                v-model="statusFilters.completed"
                @change="handleStatusFilter"
              />
              completed</label
            >
            <label
              ><input
                type="checkbox"
                v-model="statusFilters.cancelled"
                @change="handleStatusFilter"
              />
              cancelled</label
            >
          </div>
          <button @click="fetchOrders" class="refresh-btn">更新</button>
        </div>
        <div class="status-summary">
          <div class="status-group">
            <h4>waiting</h4>
            <div class="flavor-counts">
              <span class="flavor-count">つぶあん: {{ statusCounts.waiting.つぶあん }}個</span>
              <span class="flavor-count">カスタード: {{ statusCounts.waiting.カスタード }}個</span>
              <span class="flavor-count">栗きんとん: {{ statusCounts.waiting.栗きんとん }}個</span>
              <span class="flavor-count total"
                >合計:
                {{
                  statusCounts.waiting.つぶあん +
                  statusCounts.waiting.カスタード +
                  statusCounts.waiting.栗きんとん
                }}個</span
              >
            </div>
          </div>
          <div class="status-group">
            <h4>cooking</h4>
            <div class="flavor-counts">
              <span class="flavor-count">つぶあん: {{ statusCounts.cooking.つぶあん }}個</span>
              <span class="flavor-count">カスタード: {{ statusCounts.cooking.カスタード }}個</span>
              <span class="flavor-count">栗きんとん: {{ statusCounts.cooking.栗きんとん }}個</span>
              <span class="flavor-count total"
                >合計:
                {{
                  statusCounts.cooking.つぶあん +
                  statusCounts.cooking.カスタード +
                  statusCounts.cooking.栗きんとん
                }}個</span
              >
            </div>
          </div>
          <div class="status-group">
            <h4>waiting+cooking</h4>
            <div class="flavor-counts">
              <span class="flavor-count"
                >つぶあん: {{ statusCounts.waitingAndCooking.つぶあん }}個</span
              >
              <span class="flavor-count"
                >カスタード: {{ statusCounts.waitingAndCooking.カスタード }}個</span
              >
              <span class="flavor-count"
                >栗きんとん: {{ statusCounts.waitingAndCooking.栗きんとん }}個</span
              >
              <span class="flavor-count total"
                >合計:
                {{
                  statusCounts.waitingAndCooking.つぶあん +
                  statusCounts.waitingAndCooking.カスタード +
                  statusCounts.waitingAndCooking.栗きんとん
                }}個</span
              >
            </div>
          </div>
          <div class="status-group">
            <h4>stock</h4>
            <div class="flavor-counts">
              <span class="flavor-count">つぶあん: {{ stockData.つぶあん }}個</span>
              <span class="flavor-count">カスタード: {{ stockData.カスタード }}個</span>
              <span class="flavor-count">栗きんとん: {{ stockData.栗きんとん }}個</span>
              <span class="flavor-count total"
                >合計:
                {{ stockData.つぶあん + stockData.カスタード + stockData.栗きんとん }}個</span
              >
            </div>
          </div>
        </div>
        <div class="orders">
          <div v-if="orders.length === 0" class="no-orders">オーダーがありません</div>
          <div v-for="order in filteredOrders" :key="order.id" class="order-item">
            <div class="order-header">
              <span class="order-id">ID: {{ order.id }}</span>
              <span class="order-status" :class="order.status">{{ order.status }}</span>
              <span v-if="order.isPriority" class="priority-badge">優先</span>
            </div>
            <div class="order-flavors">
              <span v-for="item in order.items" :key="item.flavor">
                {{ getFlavorName(item.flavor) }}: {{ item.quantity }}個
              </span>
            </div>
            <div class="order-actions">
              <button
                v-if="['waiting', 'cooking', 'ready'].includes(order.status)"
                @click="cancelOrder(order.id)"
                class="action-btn cancel-btn"
              >
                キャンセル
              </button>
              <button
                v-if="['waiting', 'cooking'].includes(order.status)"
                @click="increasePriority(order.id)"
                class="action-btn priority-btn"
              >
                優先度上げ
              </button>
              <button
                v-if="order.status === 'ready'"
                @click="completeOrder(order.id)"
                class="action-btn complete-btn"
              >
                受け渡し完了
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- App Settings Modal -->
    <div v-if="showAppSettings" class="modal-overlay" @click="showAppSettings = false">
      <div class="modal" @click.stop>
        <h3>アプリ設定</h3>
        <div class="app-settings-form">
          <div class="form-group">
            <label>Base URL:</label>
            <input type="text" v-model="appSettings.baseUrl" placeholder="http://localhost:3000" />
          </div>
          <div class="form-group">
            <label>Token:</label>
            <input type="text" v-model="appSettings.token" placeholder="your-api-token" />
          </div>
          <div class="modal-actions">
            <button @click="saveAppSettings" class="save-btn">保存</button>
            <button @click="showAppSettings = false" class="cancel-btn">キャンセル</button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.app {
  display: flex;
  height: 100vh;
  background: #000;
  color: #fff;
  font-family: Arial, sans-serif;
  overflow: hidden;
}

.left-panel {
  width: 400px;
  background: #111;
  padding: 20px;
  border-right: 1px solid #333;
  overflow-y: auto;
  min-height: 0;
}

.right-panel {
  flex: 1;
  padding: 20px;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.button-group {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 30px;
}

.button-group button {
  padding: 15px 20px;
  background: #333;
  color: #fff;
  border: 1px solid #555;
  border-radius: 5px;
  cursor: pointer;
  transition: all 0.2s;
}

.button-group button:hover {
  background: #444;
}

.button-group button.active {
  background: #fff;
  color: #000;
}

.component-area {
  background: #222;
  padding: 20px;
  border-radius: 5px;
  min-height: 300px;
}

.order-list {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

.order-list h2 {
  margin: 0 0 20px 0;
  color: #fff;
}

.status-filters {
  display: flex;
  gap: 15px;
  margin-bottom: 20px;
  flex-wrap: wrap;
}

.status-filters label {
  display: flex;
  align-items: center;
  gap: 5px;
  color: #ccc;
  cursor: pointer;
}

.status-filters input[type='checkbox'] {
  accent-color: #fff;
}

.orders {
  border: 1px solid #333;
  border-radius: 5px;
  background: #111;
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  max-height: calc(100vh - 200px);
}

.order-item {
  padding: 10px 15px;
  border-bottom: 1px solid #333;
  color: #ccc;
}

.order-item:last-child {
  border-bottom: none;
}

.order-form h3 {
  margin: 0 0 20px 0;
  color: #fff;
}

.form-group {
  margin-bottom: 15px;
}

.form-group label {
  display: block;
  margin-bottom: 5px;
  color: #ccc;
}

.form-group select {
  width: 100%;
  padding: 8px;
  background: #333;
  color: #fff;
  border: 1px solid #555;
  border-radius: 3px;
}

.form-group input[type='checkbox'] {
  margin-right: 8px;
  accent-color: #fff;
}

.submit-btn {
  width: 100%;
  padding: 12px;
  background: #fff;
  color: #000;
  border: none;
  border-radius: 3px;
  cursor: pointer;
  font-weight: bold;
  margin-top: 10px;
}

.submit-btn:hover {
  background: #eee;
}

.baking-form h3 {
  margin: 0 0 20px 0;
  color: #fff;
}

.settings-form h3 {
  margin: 0 0 20px 0;
  color: #fff;
}

.flavor-setting {
  margin-bottom: 25px;
  padding: 15px;
  background: #333;
  border-radius: 5px;
}

.flavor-setting h4 {
  margin: 0 0 15px 0;
  color: #fff;
  font-size: 16px;
}

.setting-row {
  display: flex;
  gap: 15px;
}

.setting-row .form-group {
  flex: 1;
}

.form-group input[type='number'] {
  width: 100%;
  padding: 8px;
  background: #444;
  color: #fff;
  border: 1px solid #555;
  border-radius: 3px;
}

.no-orders {
  padding: 20px;
  text-align: center;
  color: #666;
}

.order-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 5px;
}

.order-id {
  font-weight: bold;
  color: #fff;
}

.order-status {
  padding: 2px 8px;
  border-radius: 3px;
  font-size: 12px;
  font-weight: bold;
}

.order-status.waiting {
  background: #444;
  color: #fff;
}

.order-status.cooking {
  background: #14b8a6;
  color: #fff;
}

.order-status.ready {
  background: #27ae60;
  color: #fff;
}

.order-status.completed {
  background: #95a5a6;
  color: #000;
}

.order-status.cancelled {
  background: #e74c3c;
  color: #fff;
}

.priority-badge {
  background: #e74c3c;
  color: #fff;
  padding: 2px 6px;
  border-radius: 3px;
  font-size: 11px;
  font-weight: bold;
}

.order-flavors {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  color: #ccc;
  font-size: 14px;
  margin-bottom: 10px;
}

.order-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.action-btn {
  padding: 4px 8px;
  border: none;
  border-radius: 3px;
  font-size: 12px;
  cursor: pointer;
  font-weight: bold;
}

.cancel-btn {
  background: #e74c3c;
  color: #fff;
}

.cancel-btn:hover {
  background: #c0392b;
}

.priority-btn {
  background: #3498db;
  color: #fff;
}

.priority-btn:hover {
  background: #2980b9;
}

.complete-btn {
  background: #27ae60;
  color: #fff;
}

.complete-btn:hover {
  background: #2ecc71;
}

.right-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.right-header h2 {
  margin: 0;
  color: #fff;
}

.app-settings-btn {
  padding: 8px 16px;
  background: #333;
  color: #fff;
  border: 1px solid #555;
  border-radius: 3px;
  cursor: pointer;
  font-size: 14px;
}

.app-settings-btn:hover {
  background: #444;
}

.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal {
  background: #222;
  padding: 30px;
  border-radius: 8px;
  min-width: 400px;
  border: 1px solid #333;
}

.modal h3 {
  margin: 0 0 20px 0;
  color: #fff;
  text-align: center;
}

.app-settings-form .form-group {
  margin-bottom: 20px;
}

.app-settings-form .form-group label {
  display: block;
  margin-bottom: 8px;
  color: #ccc;
  font-weight: bold;
}

.app-settings-form .form-group input[type='text'] {
  width: 100%;
  padding: 10px;
  background: #333;
  color: #fff;
  border: 1px solid #555;
  border-radius: 3px;
  box-sizing: border-box;
}

.modal-actions {
  display: flex;
  gap: 10px;
  justify-content: center;
  margin-top: 25px;
}

.save-btn {
  padding: 10px 20px;
  background: #27ae60;
  color: #fff;
  border: none;
  border-radius: 3px;
  cursor: pointer;
  font-weight: bold;
}

.save-btn:hover {
  background: #2ecc71;
}

.cancel-btn {
  padding: 10px 20px;
  background: #666;
  color: #fff;
  border: none;
  border-radius: 3px;
  cursor: pointer;
  font-weight: bold;
}

.cancel-btn:hover {
  background: #777;
}

.filters-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.refresh-btn {
  padding: 8px 16px;
  background: #27ae60;
  color: #fff;
  border: none;
  border-radius: 3px;
  cursor: pointer;
  font-weight: bold;
}

.refresh-btn:hover {
  background: #2ecc71;
}

.status-summary {
  background: #222;
  border: 1px solid #333;
  border-radius: 5px;
  padding: 15px;
  margin-bottom: 15px;
  display: flex;
  gap: 30px;
}

.status-group h4 {
  margin: 0 0 10px 0;
  color: #fff;
  font-size: 14px;
  font-weight: bold;
}

.flavor-counts {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.flavor-count {
  color: #ccc;
  font-size: 13px;
}

.flavor-count.total {
  color: #fff;
  font-weight: bold;
  border-top: 1px solid #444;
  padding-top: 5px;
  margin-top: 5px;
}
</style>

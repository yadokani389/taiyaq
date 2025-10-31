export type Flavor = 'tsubuan' | 'custard' | 'kurikinton'

export type OrderStatus = 'waiting' | 'cooking' | 'ready' | 'completed' | 'cancelled'

export type NotifyChannel = 'discord' | 'email' | 'line'

export interface Item {
  flavor: Flavor
  quantity: number
}

export interface Notify {
  channel: NotifyChannel
  target: string
}

export interface Order {
  id: number
  items: Item[]
  status: OrderStatus
  orderedAt: string
  readyAt?: string | null
  completedAt?: string | null
  isPriority: boolean
  notify?: Notify | null
}

export interface FlavorConfig {
  cookingTimeMinutes: number
  quantityPerBatch: number
}

export interface StockData {
  tsubuan: number
  custard: number
  kurikinton: number
}

export interface CreateOrderRequest {
  items: Item[]
  isPriority?: boolean
}

export interface ProductionReportRequest {
  items: Item[]
}

export interface ProductionReportResponse {
  newlyReadyOrders: number[]
  unallocatedItems: Item[]
}

export interface UpdatePriorityRequest {
  isPriority: boolean
}

export interface UpdateNotificationRequest {
  channel: NotifyChannel
  target: string
}

export interface ApiError {
  message: string
  status: number
}

export interface ApiResponse<T> {
  data?: T
  error?: ApiError
}

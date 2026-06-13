export type {
  CreateOrderRequest,
  Flavor,
  FlavorConfig,
  FlavorConfigsResponse,
  Item,
  OrderStatus,
  NotifyRequest,
  StaffOrderResponse,
  StockResponse,
  UpdateOrderPriorityRequest,
  UpdateProductionRequest,
  UpdateProductionResponse,
} from '../../../src/types/api'

export interface ApiError {
  message: string
  status: number
}

export interface ApiResponse<T> {
  data?: T
  error?: ApiError
}

import { apiClient } from '../client'
import type {
  CreateOrderRequest,
  StaffOrderResponse,
  UpdateOrderPriorityRequest,
  NotifyRequest,
  ApiResponse,
} from '../types'

export class OrdersApi {
  async getOrders(status?: string[]): Promise<ApiResponse<StaffOrderResponse[]>> {
    const params = new URLSearchParams()
    if (status && status.length > 0) {
      params.append('status', status.join(','))
    }

    const endpoint = `/api/staff/orders${params.toString() ? `?${params.toString()}` : ''}`
    return apiClient.get<StaffOrderResponse[]>(endpoint)
  }

  async createOrder(order: CreateOrderRequest): Promise<ApiResponse<StaffOrderResponse>> {
    return apiClient.post<StaffOrderResponse>('/api/staff/orders', order)
  }

  async completeOrder(id: number): Promise<ApiResponse<StaffOrderResponse>> {
    return apiClient.post<StaffOrderResponse>(`/api/staff/orders/${id}/complete`)
  }

  async cancelOrder(id: number): Promise<ApiResponse<StaffOrderResponse>> {
    return apiClient.post<StaffOrderResponse>(`/api/staff/orders/${id}/cancel`)
  }

  async updatePriority(
    id: number,
    priority: UpdateOrderPriorityRequest,
  ): Promise<ApiResponse<StaffOrderResponse>> {
    return apiClient.put<StaffOrderResponse>(`/api/staff/orders/${id}/priority`, priority)
  }

  async updateNotification(
    id: number,
    notification: NotifyRequest,
  ): Promise<ApiResponse<StaffOrderResponse>> {
    return apiClient.put<StaffOrderResponse>(`/api/staff/orders/${id}/notification`, notification)
  }
}

export const ordersApi = new OrdersApi()

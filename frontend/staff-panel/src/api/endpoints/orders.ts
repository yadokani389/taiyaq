import { apiClient } from '../client'
import type {
  Order,
  CreateOrderRequest,
  UpdatePriorityRequest,
  UpdateNotificationRequest,
  ApiResponse,
} from '../types'

export class OrdersApi {
  async getOrders(status?: string[]): Promise<ApiResponse<Order[]>> {
    const params = new URLSearchParams()
    if (status && status.length > 0) {
      params.append('status', status.join(','))
    }

    const endpoint = `/api/staff/orders${params.toString() ? `?${params.toString()}` : ''}`
    return apiClient.get<Order[]>(endpoint)
  }

  async createOrder(order: CreateOrderRequest): Promise<ApiResponse<Order>> {
    return apiClient.post<Order>('/api/staff/orders', order)
  }

  async completeOrder(id: number): Promise<ApiResponse<Order>> {
    return apiClient.post<Order>(`/api/staff/orders/${id}/complete`)
  }

  async cancelOrder(id: number): Promise<ApiResponse<Order>> {
    return apiClient.post<Order>(`/api/staff/orders/${id}/cancel`)
  }

  async updatePriority(id: number, priority: UpdatePriorityRequest): Promise<ApiResponse<Order>> {
    return apiClient.put<Order>(`/api/staff/orders/${id}/priority`, priority)
  }

  async updateNotification(
    id: number,
    notification: UpdateNotificationRequest,
  ): Promise<ApiResponse<Order>> {
    return apiClient.put<Order>(`/api/staff/orders/${id}/notification`, notification)
  }
}

export const ordersApi = new OrdersApi()

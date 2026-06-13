import { apiClient } from '../client'
import type { StockResponse } from '../types'

export const stockApi = {
  getStock: () => apiClient.get<StockResponse>('/api/staff/stock'),
}

import { apiClient } from '../client'
import type { StockData } from '../types'

export const stockApi = {
  getStock: () => apiClient.get<StockData>('/api/staff/stock'),
}

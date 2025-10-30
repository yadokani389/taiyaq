import { apiClient } from '../client'
import type { ProductionReportRequest, ProductionReportResponse, ApiResponse } from '../types'

export class ProductionApi {
  async reportProduction(
    report: ProductionReportRequest,
  ): Promise<ApiResponse<ProductionReportResponse>> {
    return apiClient.post<ProductionReportResponse>('/api/staff/production', report)
  }
}

export const productionApi = new ProductionApi()

import { apiClient } from '../client'
import type { UpdateProductionRequest, UpdateProductionResponse, ApiResponse } from '../types'

export class ProductionApi {
  async reportProduction(
    report: UpdateProductionRequest,
  ): Promise<ApiResponse<UpdateProductionResponse>> {
    return apiClient.post<UpdateProductionResponse>('/api/staff/production', report)
  }
}

export const productionApi = new ProductionApi()

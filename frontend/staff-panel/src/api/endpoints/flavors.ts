import { apiClient } from '../client'
import type { Flavor, FlavorConfig, FlavorConfigsResponse, ApiResponse } from '../types'

export class FlavorsApi {
  async getFlavorConfigs(): Promise<ApiResponse<FlavorConfigsResponse>> {
    return apiClient.get<FlavorConfigsResponse>('/api/staff/flavors/config')
  }

  async updateFlavorConfig(
    flavor: Flavor,
    config: FlavorConfig,
  ): Promise<ApiResponse<FlavorConfig>> {
    return apiClient.put<FlavorConfig>(`/api/staff/flavors/${flavor}`, config)
  }
}

export const flavorsApi = new FlavorsApi()

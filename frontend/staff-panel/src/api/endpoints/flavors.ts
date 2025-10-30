import { apiClient } from '../client'
import type { Flavor, FlavorConfig, ApiResponse } from '../types'

export class FlavorsApi {
  async updateFlavorConfig(
    flavor: Flavor,
    config: FlavorConfig,
  ): Promise<ApiResponse<FlavorConfig>> {
    return apiClient.put<FlavorConfig>(`/api/staff/flavors/${flavor}`, config)
  }
}

export const flavorsApi = new FlavorsApi()

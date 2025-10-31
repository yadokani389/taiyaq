import { apiClient } from '../client'
import type { Flavor, FlavorConfig, ApiResponse } from '../types'

export class FlavorsApi {
  async getFlavorConfigs(): Promise<ApiResponse<Record<Flavor, FlavorConfig>>> {
    return apiClient.get<Record<Flavor, FlavorConfig>>('/api/staff/flavors/config')
  }

  async updateFlavorConfig(
    flavor: Flavor,
    config: FlavorConfig,
  ): Promise<ApiResponse<FlavorConfig>> {
    return apiClient.put<FlavorConfig>(`/api/staff/flavors/${flavor}`, config)
  }
}

export const flavorsApi = new FlavorsApi()

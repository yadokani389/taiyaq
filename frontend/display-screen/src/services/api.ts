import { openApiClient } from '../../../src/services/api'
import type { DisplayOrdersResponse } from '../../../src/types/api'

export async function fetchDisplayOrders(): Promise<DisplayOrdersResponse> {
  const { data, error, response } = await openApiClient.GET('/api/orders/display')
  if (!data) {
    throw new Error(
      `Failed to fetch display orders: ${response.statusText || JSON.stringify(error)}`,
    )
  }
  return data
}

export const client = {
  fetchDisplayOrders,
}

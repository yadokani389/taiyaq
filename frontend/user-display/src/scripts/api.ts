import { openApiClient } from '../../../src/services/api'
import type {
  DisplayOrdersResponse,
  OrderDetailsResponse,
  WaitTimeResponse,
} from '../../../src/types/api'

export const fetchApiOrdersDisplay = async (): Promise<DisplayOrdersResponse> => {
  const { data, error, response } = await openApiClient.GET('/api/orders/display')
  if (!data) {
    throw new Error(`Error fetching orders: ${response.statusText || JSON.stringify(error)}`)
  }
  return data
}

export const fetchApiOrdersId = async (orderId: number): Promise<OrderDetailsResponse> => {
  const { data, error, response } = await openApiClient.GET('/api/orders/{id}', {
    params: { path: { id: orderId } },
  })
  if (!data) {
    throw new Error(
      `Error fetching order ${orderId}: ${response.statusText || JSON.stringify(error)}`,
    )
  }
  return data
}

export const fetchApiWaitTimes = async (): Promise<WaitTimeResponse> => {
  const { data, error, response } = await openApiClient.GET('/api/wait-times')
  if (!data) {
    throw new Error(`Error fetching wait times: ${response.statusText || JSON.stringify(error)}`)
  }
  return data
}

import type { DisplayResponse } from '../types/api'

const getBaseURL = () => {
  const baseURL = import.meta.env.VITE_API_BASE_URL
  if (!baseURL) {
    throw new Error('VITE_API_BASE_URL is not defined in environment variables')
  }
  return baseURL.replace(/\/+$/, '')
}

export async function fetchDisplayOrders(): Promise<DisplayResponse> {
  const base = getBaseURL()
  const response = await fetch(`${base}/api/orders/display`)
  if (!response.ok) {
    throw new Error(`Failed to fetch display orders: ${response.statusText}`)
  }
  return response.json() as Promise<DisplayResponse>
}

export const client = {
  fetchDisplayOrders,
}

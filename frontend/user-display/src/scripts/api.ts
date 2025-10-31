import type { OrdersDisplayResponse, OrdersIdResponse, WaitTimesResponse } from "@/types";

const getBaseURL = () => {
  const baseURL = import.meta.env.VITE_API_BASE_URL
  if (!baseURL) {
    throw new Error('VITE_API_BASE_URL is not defined in environment variables')
  }
  return baseURL.replace(/\/+$/, '')
};

export const fetchApiOrdersDisplay = async (): Promise<OrdersDisplayResponse> => {
  const response = await fetch(`${getBaseURL()}/api/orders/display`, {
    method: 'GET',
    headers: {
      'Content-Type': 'application/json',
    },
  });
  if (!response.ok) {
    throw new Error(`Error fetching orders: ${response.statusText}`);
  }
  return await response.json();
};

export const fetchApiOrdersId = async (orderId: number): Promise<OrdersIdResponse> => {
  const response = await fetch(`${getBaseURL()}/api/orders/${orderId}`, {
    method: 'GET',
    headers: {
      'Content-Type': 'application/json',
    },
  });
  if (!response.ok) {
    throw new Error(`Error fetching order ${orderId}: ${response.statusText}`);
  }
  return await response.json();
};

export const fetchApiWaitTimes = async (): Promise<WaitTimesResponse> => {
  const response = await fetch(`${getBaseURL()}/api/wait-times`, {
    method: 'GET',
    headers: {
      'Content-Type': 'application/json',
    },
  });
  if (!response.ok) {
    throw new Error(`Error fetching wait times: ${response.statusText}`);
  }
  return await response.json();
}

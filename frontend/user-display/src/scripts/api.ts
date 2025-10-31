import type { OrdersDisplayResponse, OrdersIdResponse } from "@/types";

const getBaseURL = () => {
  const baseURL = import.meta.env.VITE_API_BASE_URL
  if (!baseURL) {
    throw new Error('VITE_API_BASE_URL is not defined in environment variables')
  }
  return baseURL.replace(/\/+$/, '')
};

const fetchApiOrdersDisplay = async (): Promise<OrdersDisplayResponse> => {
  const response = await fetch(`${getBaseURL()}/orders/display`, {
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

const fetchApiOrdersId = async (orderId: number): Promise<OrdersIdResponse> => {
  const response = await fetch(`${getBaseURL()}/orders/${orderId}`, {
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

export { fetchApiOrdersDisplay, fetchApiOrdersId }

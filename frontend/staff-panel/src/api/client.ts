import type { ApiError, ApiResponse } from './types'

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || 'http://localhost:3000'

class ApiClient {
  private baseUrl: string
  private token: string | null = null

  constructor() {
    this.baseUrl = API_BASE_URL
    this.loadToken()
    this.loadBaseUrl()
  }

  setToken(token: string) {
    this.token = token
    localStorage.setItem('staff_token', token)
  }

  clearToken() {
    this.token = null
    localStorage.removeItem('staff_token')
  }

  setBaseUrl(baseUrl: string) {
    this.baseUrl = baseUrl
    localStorage.setItem('app_base_url', baseUrl)
  }

  getToken(): string | null {
    return this.token
  }

  getBaseUrl(): string {
    return this.baseUrl
  }

  private loadToken() {
    this.token = localStorage.getItem('staff_token')
  }

  private loadBaseUrl() {
    const savedBaseUrl = localStorage.getItem('app_base_url')
    if (savedBaseUrl) {
      this.baseUrl = savedBaseUrl
    }
  }

  private getHeaders(): Record<string, string> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    }

    if (this.token) {
      headers.Authorization = `Bearer ${this.token}`
    }

    return headers
  }

  async request<T>(endpoint: string, options: RequestInit = {}): Promise<ApiResponse<T>> {
    try {
      const url = `${this.baseUrl}${endpoint}`
      const response = await fetch(url, {
        ...options,
        headers: {
          ...this.getHeaders(),
          ...options.headers,
        },
      })

      if (!response.ok) {
        const errorData = await response.text()
        return {
          error: {
            message: errorData || `HTTP ${response.status}`,
            status: response.status,
          },
        }
      }

      if (response.status === 204) {
        return { data: undefined as T }
      }

      const data = await response.json()
      return { data }
    } catch (error) {
      return {
        error: {
          message: error instanceof Error ? error.message : 'Network error',
          status: 0,
        },
      }
    }
  }

  async get<T>(endpoint: string): Promise<ApiResponse<T>> {
    return this.request<T>(endpoint, { method: 'GET' })
  }

  async post<T>(endpoint: string, body?: unknown): Promise<ApiResponse<T>> {
    return this.request<T>(endpoint, {
      method: 'POST',
      body: body ? JSON.stringify(body) : undefined,
    })
  }

  async put<T>(endpoint: string, body?: unknown): Promise<ApiResponse<T>> {
    return this.request<T>(endpoint, {
      method: 'PUT',
      body: body ? JSON.stringify(body) : undefined,
    })
  }

  async delete<T>(endpoint: string): Promise<ApiResponse<T>> {
    return this.request<T>(endpoint, { method: 'DELETE' })
  }
}

export const apiClient = new ApiClient()

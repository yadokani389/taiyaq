import { apiClient } from './client'

export interface AuthState {
  isAuthenticated: boolean
  token: string | null
}

export class AuthManager {
  private static instance: AuthManager
  private _isAuthenticated = false

  static getInstance(): AuthManager {
    if (!AuthManager.instance) {
      AuthManager.instance = new AuthManager()
    }
    return AuthManager.instance
  }

  constructor() {
    this.checkExistingToken()
  }

  get isAuthenticated(): boolean {
    return this._isAuthenticated
  }

  private checkExistingToken() {
    const token = localStorage.getItem('staff_token')
    if (token) {
      this._isAuthenticated = true
      apiClient.setToken(token)
    }
  }

  login(token: string): boolean {
    try {
      apiClient.setToken(token)
      this._isAuthenticated = true
      return true
    } catch (error) {
      console.error('Failed to login:', error)
      return false
    }
  }

  logout(): void {
    apiClient.clearToken()
    this._isAuthenticated = false
  }

  async validateToken(): Promise<boolean> {
    if (!this._isAuthenticated) {
      return false
    }

    try {
      const response = await apiClient.get('/api/staff/orders')
      if (response.error?.status === 401) {
        this.logout()
        return false
      }
      return true
    } catch (error) {
      this.logout()
      return false
    }
  }
}

export const authManager = AuthManager.getInstance()

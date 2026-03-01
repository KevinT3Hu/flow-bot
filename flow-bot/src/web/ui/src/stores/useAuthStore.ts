import { create } from 'zustand'

const API_BASE = '/api'

interface AuthState {
  token: string | null
  isAuthenticated: boolean
  authEnabled: boolean
  isLoading: boolean
  error: string | null
  checkAuth: () => Promise<void>
  login: (password: string) => Promise<{ success: boolean; message?: string }>
  logout: () => void
  getAuthHeaders: () => Record<string, string>
  getWebSocketUrl: () => string
}

export const useAuthStore = create<AuthState>((set, get) => ({
  token: localStorage.getItem('flowbot_token') || null,
  isAuthenticated: false,
  authEnabled: false,
  isLoading: true,
  error: null,

  checkAuth: async () => {
    const { token } = get()

    try {
      const response = await fetch(`${API_BASE}/info`, {
        headers: token ? { Authorization: `Bearer ${token}` } : {},
      })

      const data = await response.json()

      if (response.status === 401) {
        set({ authEnabled: true, isAuthenticated: false, isLoading: false })
        return
      }

      if (response.ok) {
        set({
          authEnabled: data.auth_enabled,
          isAuthenticated: !data.auth_enabled || !!token,
          isLoading: false,
          error: null,
        })
      }
    } catch (err) {
      set({ error: (err as Error).message, isLoading: false })
    }
  },

  login: async (password: string) => {
    set({ isLoading: true, error: null })

    try {
      const response = await fetch(`${API_BASE}/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ password }),
      })

      const data = await response.json()

      if (data.success && data.token) {
        localStorage.setItem('flowbot_token', data.token)
        set({
          token: data.token,
          isAuthenticated: true,
          isLoading: false,
          error: null,
        })
        return { success: true }
      } else {
        set({
          isLoading: false,
          error: data.message || 'Login failed',
        })
        return { success: false, message: data.message }
      }
    } catch (err) {
      set({
        isLoading: false,
        error: (err as Error).message,
      })
      return { success: false, message: (err as Error).message }
    }
  },

  logout: () => {
    localStorage.removeItem('flowbot_token')
    set({
      token: null,
      isAuthenticated: false,
    })
  },

  getAuthHeaders: () => {
    const { token } = get()
    return token ? { Authorization: `Bearer ${token}` } : {}
  },

  getWebSocketUrl: () => {
    const { token } = get()
    const wsUrl = `ws://${window.location.host}/ws/logs`
    return token ? `${wsUrl}?token=${encodeURIComponent(token)}` : wsUrl
  },
}))

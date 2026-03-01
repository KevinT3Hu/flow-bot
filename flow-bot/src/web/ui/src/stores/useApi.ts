import { useAuthStore } from './useAuthStore'

const API_BASE = '/api'

export async function fetchWithAuth(url: string, options: RequestInit = {}): Promise<Response> {
  const authHeaders = useAuthStore.getState().getAuthHeaders()

  const response = await fetch(url, {
    ...options,
    headers: {
      ...options.headers,
      ...authHeaders,
      'Content-Type': 'application/json',
    },
  })

  if (response.status === 401) {
    useAuthStore.getState().logout()
    throw new Error('Unauthorized')
  }

  return response
}

interface BotInfo {
  version: string
  uptime_seconds: number
  connection_mode: string
  plugin_count: number
  total_plugins_in_dir: number
  auth_enabled: boolean
}

export const useBotApi = () => {
  const fetchInfo = async (): Promise<BotInfo> => {
    const response = await fetchWithAuth(`${API_BASE}/info`)
    if (!response.ok) throw new Error('Failed to fetch bot info')
    return response.json()
  }

  return { fetchInfo }
}

export interface Plugin {
  name: string
  version: string
  description: string
  enabled: boolean
  loaded_at?: string
}

interface ApiResponse {
  success: boolean
  message: string
}

export const usePluginApi = () => {
  const fetchPlugins = async (): Promise<Plugin[]> => {
    const response = await fetchWithAuth(`${API_BASE}/plugins`)
    if (!response.ok) throw new Error('Failed to fetch plugins')
    return response.json()
  }

  const enablePlugin = async (name: string): Promise<ApiResponse> => {
    const response = await fetchWithAuth(
      `${API_BASE}/plugins/${encodeURIComponent(name)}/enable`,
      { method: 'POST' }
    )
    return response.json()
  }

  const disablePlugin = async (name: string): Promise<ApiResponse> => {
    const response = await fetchWithAuth(
      `${API_BASE}/plugins/${encodeURIComponent(name)}/disable`,
      { method: 'POST' }
    )
    return response.json()
  }

  return { fetchPlugins, enablePlugin, disablePlugin }
}

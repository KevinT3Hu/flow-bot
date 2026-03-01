import { create } from 'zustand'
import { fetchWithAuth } from './useApi'

const API_BASE = '/api'

interface BotInfo {
  version: string
  uptime_seconds: number
  connection_mode: string
  plugin_count: number
  total_plugins_in_dir: number
  auth_enabled: boolean
}

interface BotState {
  info: BotInfo | null
  isLoading: boolean
  error: string | null
  fetchInfo: () => Promise<void>
  startPolling: () => () => void
}

export const useBotStore = create<BotState>((set, get) => ({
  info: null,
  isLoading: true,
  error: null,

  fetchInfo: async () => {
    try {
      const response = await fetchWithAuth(`${API_BASE}/info`)
      if (!response.ok) throw new Error('Failed to fetch bot info')
      const data: BotInfo = await response.json()
      set({ info: data, error: null })
    } catch (err) {
      set({ error: (err as Error).message })
    } finally {
      set({ isLoading: false })
    }
  },

  startPolling: () => {
    const { fetchInfo } = get()
    fetchInfo()
    const interval = setInterval(fetchInfo, 5000)
    return () => clearInterval(interval)
  },
}))

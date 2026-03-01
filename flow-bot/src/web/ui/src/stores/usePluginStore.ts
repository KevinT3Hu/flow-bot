import { create } from 'zustand'
import { fetchWithAuth, type Plugin } from './useApi'

const API_BASE = '/api'

interface ApiResponse {
  success: boolean
  message: string
}

interface PluginState {
  plugins: Plugin[]
  isLoading: boolean
  error: string | null
  processingPlugins: Set<string>
  enabledCount: number
  fetchPlugins: () => Promise<void>
  enablePlugin: (name: string) => Promise<{ success: boolean; message: string }>
  disablePlugin: (name: string) => Promise<{ success: boolean; message: string }>
}

export const usePluginStore = create<PluginState>((set, get) => ({
  plugins: [],
  isLoading: false,
  error: null,
  processingPlugins: new Set(),

  get enabledCount() {
    return get().plugins.filter((p) => p.enabled).length
  },

  fetchPlugins: async () => {
    set({ isLoading: true, error: null })
    try {
      const response = await fetchWithAuth(`${API_BASE}/plugins`)
      if (!response.ok) throw new Error('Failed to fetch plugins')
      const data: Plugin[] = await response.json()
      set({ plugins: data })
    } catch (err) {
      set({ error: (err as Error).message })
    } finally {
      set({ isLoading: false })
    }
  },

  enablePlugin: async (name: string) => {
    const { processingPlugins } = get()
    set({ processingPlugins: new Set([...processingPlugins, name]) })

    try {
      const response = await fetchWithAuth(
        `${API_BASE}/plugins/${encodeURIComponent(name)}/enable`,
        { method: 'POST' }
      )
      const data: ApiResponse = await response.json()
      if (!response.ok) throw new Error(data.error || 'Failed to enable plugin')

      await get().fetchPlugins()
      return { success: true, message: data.message }
    } catch (err) {
      return { success: false, message: (err as Error).message }
    } finally {
      const currentProcessing = get().processingPlugins
      const newProcessing = new Set(currentProcessing)
      newProcessing.delete(name)
      set({ processingPlugins: newProcessing })
    }
  },

  disablePlugin: async (name: string) => {
    const { processingPlugins } = get()
    set({ processingPlugins: new Set([...processingPlugins, name]) })

    try {
      const response = await fetchWithAuth(
        `${API_BASE}/plugins/${encodeURIComponent(name)}/disable`,
        { method: 'POST' }
      )
      const data: ApiResponse = await response.json()
      if (!response.ok) throw new Error(data.error || 'Failed to disable plugin')

      await get().fetchPlugins()
      return { success: true, message: data.message }
    } catch (err) {
      return { success: false, message: (err as Error).message }
    } finally {
      const currentProcessing = get().processingPlugins
      const newProcessing = new Set(currentProcessing)
      newProcessing.delete(name)
      set({ processingPlugins: newProcessing })
    }
  },
}))

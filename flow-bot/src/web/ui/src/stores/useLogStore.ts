import { create } from 'zustand'
import { useAuthStore } from './useAuthStore'

const MAX_LOGS = 1000

interface LogMessage {
  timestamp: string
  level: string
  target: string
  message: string
}

interface LogFilters {
  info: boolean
  warn: boolean
  error: boolean
  debug: boolean
  trace: boolean
}

interface LogState {
  logs: LogMessage[]
  isConnected: boolean
  filters: LogFilters
  autoScroll: boolean
  ws: WebSocket | null
  reconnectTimeout: ReturnType<typeof setTimeout> | null
  filteredLogs: LogMessage[]
  connect: () => void
  disconnect: () => void
  clearLogs: () => void
  toggleFilter: (key: keyof LogFilters) => void
  setAutoScroll: (value: boolean) => void
}

export const useLogStore = create<LogState>((set, get) => ({
  logs: [],
  isConnected: false,
  filters: {
    info: true,
    warn: true,
    error: true,
    debug: false,
    trace: false,
  },
  autoScroll: true,
  ws: null,
  reconnectTimeout: null,

  get filteredLogs() {
    const { logs, filters } = get()
    return logs.filter((log) => {
      const level = log.level.toLowerCase() as keyof LogFilters
      return filters[level] !== false
    })
  },

  connect: () => {
    const { ws, reconnectTimeout } = get()

    if (ws?.readyState === WebSocket.OPEN) return

    if (reconnectTimeout) {
      clearTimeout(reconnectTimeout)
      set({ reconnectTimeout: null })
    }

    try {
      const wsUrl = useAuthStore.getState().getWebSocketUrl()
      const newWs = new WebSocket(wsUrl)
      set({ ws: newWs })

      newWs.onopen = () => {
        set({ isConnected: true })
      }

      newWs.onmessage = (event) => {
        try {
          const log: LogMessage = JSON.parse(event.data)
          set((state) => {
            const newLogs = [...state.logs, log]
            if (newLogs.length > MAX_LOGS) {
              return { logs: newLogs.slice(newLogs.length - MAX_LOGS) }
            }
            return { logs: newLogs }
          })
        } catch (err) {
          console.error('Failed to parse log message:', err)
        }
      }

      newWs.onclose = () => {
        set({ isConnected: false, ws: null })
        const timeout = setTimeout(() => get().connect(), 3000)
        set({ reconnectTimeout: timeout })
      }

      newWs.onerror = (error) => {
        console.error('WebSocket error:', error)
        newWs.close()
      }
    } catch (err) {
      console.error('Failed to create WebSocket:', err)
      const timeout = setTimeout(() => get().connect(), 3000)
      set({ reconnectTimeout: timeout })
    }
  },

  disconnect: () => {
    const { ws, reconnectTimeout } = get()
    if (reconnectTimeout) clearTimeout(reconnectTimeout)
    if (ws) ws.close()
    set({ ws: null, reconnectTimeout: null })
  },

  clearLogs: () => set({ logs: [] }),

  toggleFilter: (key: keyof LogFilters) =>
    set((state) => ({
      filters: { ...state.filters, [key]: !state.filters[key] },
    })),

  setAutoScroll: (value: boolean) => set({ autoScroll: value }),
}))

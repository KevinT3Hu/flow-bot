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

const filterLogs = (logs: LogMessage[], filters: LogFilters): LogMessage[] => {
  return logs.filter((log) => {
    const level = log.level.toLowerCase() as keyof LogFilters
    return filters[level] !== false
  })
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
  filteredLogs: [],

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
            const trimmedLogs = newLogs.length > MAX_LOGS
              ? newLogs.slice(newLogs.length - MAX_LOGS)
              : newLogs
            return {
              logs: trimmedLogs,
              filteredLogs: filterLogs(trimmedLogs, state.filters)
            }
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

  clearLogs: () => set((state) => ({
    logs: [],
    filteredLogs: filterLogs([], state.filters)
  })),

  toggleFilter: (key: keyof LogFilters) =>
    set((state) => {
      const newFilters = { ...state.filters, [key]: !state.filters[key] }
      return {
        filters: newFilters,
        filteredLogs: filterLogs(state.logs, newFilters)
      }
    }),

  setAutoScroll: (value: boolean) => set({ autoScroll: value }),
}))

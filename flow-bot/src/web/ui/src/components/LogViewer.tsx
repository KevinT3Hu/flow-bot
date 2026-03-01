import { useRef, useEffect } from 'react'
import { useLogStore } from '@/stores/useLogStore'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Terminal,
  Trash2,
  Download,
  Wifi,
  WifiOff,
} from 'lucide-react'
import { formatTime } from '@/utils/format'

interface LogLevel {
  key: 'info' | 'warn' | 'error' | 'debug' | 'trace'
  label: string
  className: string
}

const LOG_LEVELS: LogLevel[] = [
  { key: 'info', label: 'Info', className: 'log-level-info' },
  { key: 'warn', label: 'Warn', className: 'log-level-warn' },
  { key: 'error', label: 'Error', className: 'log-level-error' },
  { key: 'debug', label: 'Debug', className: 'log-level-debug' },
  { key: 'trace', label: 'Trace', className: 'log-level-trace' },
]

export default function LogViewer() {
  const {
    filteredLogs,
    isConnected,
    filters,
    autoScroll,
    toggleFilter,
    clearLogs,
    setAutoScroll,
  } = useLogStore()

  const logContainerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (autoScroll && logContainerRef.current) {
      logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight
    }
  }, [filteredLogs, autoScroll])

  const handleExport = () => {
    const content = filteredLogs
      .map((log) => `[${log.timestamp}] ${log.level}: ${log.message}`)
      .join('\n')
    const blob = new Blob([content], { type: 'text/plain' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `flow-bot-logs-${new Date().toISOString()}.txt`
    a.click()
    URL.revokeObjectURL(url)
  }

  const getLevelClass = (level: string): string => {
    const classes: Record<string, string> = {
      info: 'log-level-info',
      warn: 'log-level-warn',
      error: 'log-level-error',
      debug: 'log-level-debug',
      trace: 'log-level-trace',
    }
    return classes[level.toLowerCase()] || 'log-level-trace'
  }

  return (
    <Card className="animate-slide-in">
      <CardHeader className="flex flex-row items-center justify-between">
        <div className="flex items-center gap-3">
          <Terminal className="h-5 w-5 text-primary" />
          <CardTitle>Realtime Logs</CardTitle>
          <div className={`flex items-center gap-2 px-2 py-1 rounded-md ${isConnected ? 'bg-green-500/20 text-green-500' : 'bg-red-500/20 text-red-500'}`}>
            {isConnected ? <Wifi className="h-4 w-4" /> : <WifiOff className="h-4 w-4" />}
          </div>
        </div>

        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={handleExport}>
            <Download className="h-4 w-4 mr-1" />
            Export
          </Button>
          <Button variant="outline" size="sm" onClick={clearLogs}>
            <Trash2 className="h-4 w-4 mr-1" />
            Clear
          </Button>
        </div>
      </CardHeader>

      <CardContent className="space-y-4">
        <div className="flex flex-wrap items-center justify-between gap-4 pb-4 border-b">
          <div className="flex items-center gap-4">
            {LOG_LEVELS.map(({ key, label }) => (
              <label key={key} className="flex items-center gap-2 cursor-pointer">
                <Checkbox
                  checked={filters[key]}
                  onCheckedChange={() => toggleFilter(key)}
                />
                <span className="text-sm">{label}</span>
              </label>
            ))}
          </div>

          <label className="flex items-center gap-2 cursor-pointer">
            <Checkbox
              checked={autoScroll}
              onCheckedChange={(checked) => setAutoScroll(checked as boolean)}
            />
            <span className="text-sm">Auto-scroll</span>
          </label>
        </div>

        <div
          ref={logContainerRef}
          className="bg-secondary/50 rounded-md p-4 h-[400px] overflow-y-auto font-mono text-sm"
        >
          {filteredLogs.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
              <Terminal className="h-12 w-12 mb-4 opacity-50" />
              <p>No logs to display</p>
              <p className="text-sm">
                {isConnected
                  ? 'Waiting for new log messages...'
                  : 'WebSocket disconnected. Reconnecting...'}
              </p>
            </div>
          ) : (
            filteredLogs.map((log, index) => (
              <div key={index} className="flex items-center gap-3 py-1 border-b border-border/50 last:border-0">
                <span className="text-muted-foreground text-xs shrink-0">
                  {formatTime(log.timestamp)}
                </span>
                <span className={`text-xs font-semibold uppercase px-1.5 py-0.5 rounded border ${getLevelClass(log.level)}`}>
                  {log.level}
                </span>
                <span className="text-muted-foreground text-xs shrink-0">[{log.target}]</span>
                <span className="break-all">{log.message}</span>
              </div>
            ))
          )}
        </div>
      </CardContent>
    </Card>
  )
}

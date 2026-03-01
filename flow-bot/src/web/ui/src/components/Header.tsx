import { Bot, Clock, Globe, Activity, LogOut } from 'lucide-react'
import { Card, CardHeader } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { useAuthStore } from '@/stores/useAuthStore'
import { formatDuration } from '@/utils/format'

interface BotInfo {
  version: string
  uptime_seconds: number
  connection_mode: string
  plugin_count: number
  total_plugins_in_dir: number
  auth_enabled: boolean
}

interface HeaderProps {
  info: BotInfo | null
  isConnected: boolean
}

export default function Header({ info, isConnected }: HeaderProps) {
  const { logout } = useAuthStore()

  return (
    <Card className="animate-fade-in border-primary/20">
      <CardHeader className="flex flex-row items-center justify-between">
        <div className="flex items-center gap-4">
          <div className="h-12 w-12 rounded-lg bg-gradient-to-br from-primary to-purple-600 flex items-center justify-center text-primary-foreground shadow-lg shadow-primary/30">
            <Bot className="h-6 w-6" />
          </div>
          <div>
            <h1 className="text-2xl font-bold">Flow-Bot</h1>
            <p className="text-sm text-muted-foreground">Web Management Interface</p>
          </div>
        </div>

        <div className="flex items-center gap-6">
          {info && (
            <>
              <div className="flex flex-col gap-1">
                <span className="text-xs text-muted-foreground uppercase tracking-wider">Version</span>
                <span className="font-semibold">v{info.version}</span>
              </div>

              <div className="flex flex-col gap-1">
                <span className="text-xs text-muted-foreground uppercase tracking-wider">Uptime</span>
                <div className="flex items-center gap-2">
                  <Clock className="h-4 w-4" />
                  <span className="font-semibold">{formatDuration(info.uptime_seconds)}</span>
                </div>
              </div>

              <div className="flex flex-col gap-1">
                <span className="text-xs text-muted-foreground uppercase tracking-wider">Mode</span>
                <div className="flex items-center gap-2">
                  <Globe className="h-4 w-4" />
                  <span className="font-semibold capitalize">{info.connection_mode}</span>
                </div>
              </div>

              <div className="flex flex-col gap-1">
                <span className="text-xs text-muted-foreground uppercase tracking-wider">Status</span>
                <div className="flex items-center gap-2">
                  <Activity className={`h-4 w-4 ${isConnected ? 'text-green-500' : 'text-red-500'}`} />
                  <span className={`font-semibold ${isConnected ? 'text-green-500' : 'text-red-500'}`}>
                    {isConnected ? 'Connected' : 'Disconnected'}
                  </span>
                </div>
              </div>
            </>
          )}

          <Button variant="outline" size="icon" onClick={logout} title="Logout">
            <LogOut className="h-4 w-4" />
          </Button>
        </div>
      </CardHeader>
    </Card>
  )
}

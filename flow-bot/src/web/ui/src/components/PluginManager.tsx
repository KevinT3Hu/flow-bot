import { usePluginStore } from '@/stores/usePluginStore'
import { useToastStore } from '@/stores/useToastStore'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import {
  Puzzle,
  RefreshCw,
  Power,
  PowerOff,
  CheckCircle2,
  XCircle,
  Loader2,
} from 'lucide-react'

export default function PluginManager() {
  const { plugins, isLoading, processingPlugins, enabledCount, fetchPlugins, enablePlugin, disablePlugin } =
    usePluginStore()
  const addToast = useToastStore((state) => state.addToast)

  const handleEnable = async (name: string) => {
    const result = await enablePlugin(name)
    addToast(result.message, result.success ? 'success' : 'error')
  }

  const handleDisable = async (name: string) => {
    const result = await disablePlugin(name)
    addToast(result.message, result.success ? 'success' : 'error')
  }

  return (
    <Card className="animate-slide-in">
      <CardHeader className="flex flex-row items-center justify-between">
        <div className="flex items-center gap-3">
          <Puzzle className="h-5 w-5 text-primary" />
          <CardTitle>Plugin Manager</CardTitle>
          <Badge variant="secondary">
            {enabledCount}/{plugins.length} Active
          </Badge>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={fetchPlugins}
          disabled={isLoading}
        >
          {isLoading ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <RefreshCw className="h-4 w-4" />
          )}
          Refresh
        </Button>
      </CardHeader>

      <CardContent>
        {plugins.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
            <Puzzle className="h-12 w-12 mb-4 opacity-50" />
            <p>No plugins found</p>
            <p className="text-sm">Place .wasm plugin files in the plugins directory</p>
          </div>
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Plugin</TableHead>
                <TableHead>Version</TableHead>
                <TableHead>Description</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {plugins.map((plugin) => (
                <TableRow key={plugin.name}>
                  <TableCell>
                    <div className="flex items-center gap-2">
                      <Puzzle className="h-4 w-4 text-muted-foreground" />
                      <span className="font-medium">{plugin.name}</span>
                    </div>
                  </TableCell>
                  <TableCell className="text-muted-foreground">
                    {plugin.version || '-'}
                  </TableCell>
                  <TableCell className="text-muted-foreground max-w-[300px] truncate">
                    {plugin.description || 'No description'}
                  </TableCell>
                  <TableCell>
                    {plugin.enabled ? (
                      <Badge className="bg-green-500/20 text-green-500 hover:bg-green-500/30">
                        <CheckCircle2 className="h-3 w-3 mr-1" />
                        Enabled
                      </Badge>
                    ) : (
                      <Badge variant="destructive">
                        <XCircle className="h-3 w-3 mr-1" />
                        Disabled
                      </Badge>
                    )}
                  </TableCell>
                  <TableCell>
                    {plugin.enabled ? (
                      <Button
                        variant="destructive"
                        size="sm"
                        onClick={() => handleDisable(plugin.name)}
                        disabled={processingPlugins.has(plugin.name)}
                      >
                        {processingPlugins.has(plugin.name) ? (
                          <Loader2 className="h-4 w-4 animate-spin" />
                        ) : (
                          <PowerOff className="h-4 w-4 mr-1" />
                        )}
                        Disable
                      </Button>
                    ) : (
                      <Button
                        size="sm"
                        onClick={() => handleEnable(plugin.name)}
                        disabled={processingPlugins.has(plugin.name)}
                        className="bg-green-600 hover:bg-green-700"
                      >
                        {processingPlugins.has(plugin.name) ? (
                          <Loader2 className="h-4 w-4 animate-spin" />
                        ) : (
                          <Power className="h-4 w-4 mr-1" />
                        )}
                        Enable
                      </Button>
                    )}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </CardContent>
    </Card>
  )
}

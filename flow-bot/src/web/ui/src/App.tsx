import { useEffect } from 'react'
import { useAuthStore } from '@/stores/useAuthStore'
import { useBotStore } from '@/stores/useBotStore'
import { useLogStore } from '@/stores/useLogStore'
import { usePluginStore } from '@/stores/usePluginStore'
import Header from '@/components/Header'
import PluginManager from '@/components/PluginManager'
import LogViewer from '@/components/LogViewer'
import ToastContainer from '@/components/ToastContainer'
import LoginPage from '@/components/LoginPage'
import { Loader2 } from 'lucide-react'

function App() {
  const { isAuthenticated, isLoading, checkAuth } = useAuthStore()

  // Check auth status on mount
  useEffect(() => {
    checkAuth()
  }, [])

  // Initialize stores after authentication
  useEffect(() => {
    if (!isAuthenticated) return

    const stopPolling = useBotStore.getState().startPolling()
    usePluginStore.getState().fetchPlugins()
    useLogStore.getState().connect()

    return () => {
      stopPolling()
      useLogStore.getState().disconnect()
    }
  }, [isAuthenticated])

  if (isLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-background">
        <Loader2 className="h-8 w-8 animate-spin text-primary" />
      </div>
    )
  }

  if (!isAuthenticated) {
    return <LoginPage />
  }

  const info = useBotStore((state) => state.info)
  const isConnected = useLogStore((state) => state.isConnected)

  return (
    <div className="min-h-screen bg-background text-foreground">
      <div className="container mx-auto p-6 space-y-6">
        <Header info={info} isConnected={isConnected} />

        <div className="grid gap-6">
          <PluginManager />
          <LogViewer />
        </div>
      </div>

      <ToastContainer />
    </div>
  )
}

export default App

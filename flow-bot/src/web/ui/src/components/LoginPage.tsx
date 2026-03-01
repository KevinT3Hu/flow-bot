import { useState } from 'react'
import { useAuthStore } from '@/stores/useAuthStore'
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Bot, Lock, Loader2 } from 'lucide-react'

export default function LoginPage() {
  const [password, setPassword] = useState('')
  const { login, isLoading, error, authEnabled } = useAuthStore()

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    await login(password)
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-background p-4">
      <Card className="w-full max-w-md">
        <CardHeader className="text-center">
          <div className="mx-auto h-16 w-16 rounded-lg bg-gradient-to-br from-primary to-purple-600 flex items-center justify-center text-primary-foreground shadow-lg shadow-primary/30 mb-4">
            <Bot className="h-8 w-8" />
          </div>
          <CardTitle className="text-2xl">Flow-Bot</CardTitle>
          <CardDescription>
            {authEnabled
              ? 'Enter your password to access the dashboard'
              : 'Web Management Interface'}
          </CardDescription>
        </CardHeader>

        <CardContent>
          {authEnabled ? (
            <form onSubmit={handleSubmit} className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="password">Password</Label>
                <div className="relative">
                  <Lock className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                  <Input
                    id="password"
                    type="password"
                    placeholder="Enter your password"
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    className="pl-10"
                    disabled={isLoading}
                  />
                </div>
              </div>

              {error && (
                <div className="text-sm text-red-500 text-center">{error}</div>
              )}

              <Button type="submit" className="w-full" disabled={isLoading || !password}>
                {isLoading ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Signing in...
                  </>
                ) : (
                  'Sign In'
                )}
              </Button>
            </form>
          ) : (
            <div className="text-center text-muted-foreground">
              <p>No authentication required.</p>
              <Button onClick={() => useAuthStore.setState({ isAuthenticated: true })} className="mt-4">
                Continue
              </Button>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  )
}

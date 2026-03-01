import { useToastStore, type ToastType } from '@/stores/useToastStore'
import { X, CheckCircle, AlertCircle, Info } from 'lucide-react'

const icons: Record<ToastType, typeof CheckCircle> = {
  success: CheckCircle,
  error: AlertCircle,
  info: Info,
}

export default function ToastContainer() {
  const { toasts, removeToast } = useToastStore()

  if (toasts.length === 0) return null

  return (
    <div className="fixed bottom-6 right-6 flex flex-col gap-3 z-50">
      {toasts.map((toast) => {
        const Icon = icons[toast.type]
        const bgClass = {
          success: 'bg-green-500',
          error: 'bg-red-500',
          info: 'bg-blue-500',
        }[toast.type] || 'bg-gray-500'

        return (
          <div
            key={toast.id}
            className={`${bgClass} text-white px-4 py-3 rounded-lg shadow-lg flex items-center gap-3 min-w-[300px] animate-slide-in`}
          >
            <Icon className="h-5 w-5 shrink-0" />
            <span className="flex-1">{toast.message}</span>
            <button
              onClick={() => removeToast(toast.id)}
              className="opacity-70 hover:opacity-100 p-1 hover:bg-white/10 rounded"
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        )
      })}
    </div>
  )
}

import { Component, ErrorInfo, ReactNode } from "react"

interface Props {
  children: ReactNode
  fallback?: ReactNode
}

interface State {
  hasError: boolean
  error: Error | null
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props)
    this.state = { hasError: false, error: null }
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("Error caught:", error, errorInfo)
  }

  render() {
    if (this.state.hasError) {
      return this.props.fallback || (
        <div className="flex items-center justify-center h-full p-4">
          <div className="text-center">
            <h2 className="text-lg font-semibold text-red-600 mb-2">出错了</h2>
            <p className="text-sm text-gray-600 mb-4">
              {this.state.error?.message || "未知错误"}
            </p>
            <button
              onClick={() => this.setState({ hasError: false, error: null })}
              className="px-4 py-2 bg-violet-600 text-white rounded-md"
            >
              重试
            </button>
          </div>
        </div>
      )
    }

    return this.props.children
  }
}
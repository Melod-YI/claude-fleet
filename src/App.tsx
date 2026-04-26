import { AppLayout } from "@/components/layout"
import { useSessions } from "@/hooks"
import { useEffect } from "react"

function App() {
  const { sessions, loading, error } = useSessions()

  useEffect(() => {
    console.log("Sessions:", sessions)
  }, [sessions])

  return (
    <AppLayout>
      <div className="flex items-center justify-center h-full text-muted-foreground">
        {loading && "加载中..."}
        {error && `错误: ${error}`}
        {!loading && !error && `已加载 ${sessions.length} 个 session`}
      </div>
    </AppLayout>
  )
}

export default App
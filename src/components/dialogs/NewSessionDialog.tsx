import { useState } from "react"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { FolderOpen, Loader2 } from "lucide-react"
import { open as openDialog } from "@tauri-apps/plugin-dialog"
import type { FavoritePath } from "@/types"
import { useSettingsStore } from "@/stores/settingsStore"
import { startNewSession } from "@/services/sessionLaunchService"
import { PathCard } from "./PathCard"

interface NewSessionDialogProps {
  open: boolean
  onClose: () => void
  favoritePaths: FavoritePath[]
  onRecordPathUsage: (path: string) => void
}

export function NewSessionDialog({
  open,
  onClose,
  favoritePaths,
  onRecordPathUsage,
}: NewSessionDialogProps) {
  const [workingDirectory, setWorkingDirectory] = useState("")
  const [sessionName, setSessionName] = useState("")
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const togglePinPath = useSettingsStore((state) => state.togglePinPath)
  const removeFavoritePath = useSettingsStore((state) => state.removeFavoritePath)

  const handleBrowse = async () => {
    try {
      const selectedPath = await openDialog({
        directory: true,
        multiple: false,
      })
      if (selectedPath) {
        setWorkingDirectory(selectedPath as string)
        setSessionName("") // 清空名称，使用默认
      }
    } catch (e) {
      console.error("打开文件夹选择失败:", e)
    }
  }

  const handleSelectFavoritePath = (path: string) => {
    setWorkingDirectory(path)
    setSessionName("")
  }

  const handleStart = async () => {
    if (!workingDirectory.trim()) {
      setError("请选择工作目录")
      return
    }

    setLoading(true)
    setError(null)

    try {
      await startNewSession({
        workingDirectory,
        name: sessionName.trim() || undefined,
      })

      // 记录路径使用（用于排序）
      onRecordPathUsage(workingDirectory)
      onClose()

      // 刷新 session 列表
      // Phase 7 实现钩子通知后会自动更新
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>新建 Session</DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-4 py-4">
          {/* 工作目录 */}
          <div className="flex flex-col gap-2">
            <label className="text-sm font-medium text-gray-700">工作目录</label>
            <div className="flex gap-2">
              <Input
                value={workingDirectory}
                onChange={(e) => setWorkingDirectory(e.target.value)}
                placeholder="选择或输入路径..."
                className="flex-1"
              />
              <Button
                variant="outline"
                size="sm"
                onClick={handleBrowse}
              >
                <FolderOpen className="w-4 h-4" />
                浏览...
              </Button>
            </div>
          </div>

          {/* Session 名称 */}
          <div className="flex flex-col gap-2">
            <label className="text-sm font-medium text-gray-700">
              Session 名称（可选）
            </label>
            <Input
              value={sessionName}
              onChange={(e) => setSessionName(e.target.value)}
              placeholder="默认使用目录名称"
            />
          </div>

          {/* 常用路径 */}
          {favoritePaths.length > 0 && (
            <div className="flex flex-col gap-2">
              <label className="text-sm font-medium text-gray-700">常用路径</label>
              <div className="flex flex-wrap gap-2">
                {favoritePaths.map((fp) => (
                  <PathCard
                    key={fp.path}
                    path={fp}
                    onPinToggle={() => togglePinPath(fp.path)}
                    onDelete={() => removeFavoritePath(fp.path)}
                    onSelect={() => handleSelectFavoritePath(fp.path)}
                    isSelected={workingDirectory.trim() !== "" && fp.path.toLowerCase() === workingDirectory.trim().toLowerCase()}
                  />
                ))}
              </div>
            </div>
          )}

          {/* 错误提示 */}
          {error && (
            <div className="text-sm text-red-500">{error}</div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            取消
          </Button>
          <Button
            variant="default"
            onClick={handleStart}
            disabled={loading}
            className="bg-violet-600 hover:bg-violet-700"
          >
            {loading ? (
              <>
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                启动中...
              </>
            ) : (
              "启动 Claude Code"
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
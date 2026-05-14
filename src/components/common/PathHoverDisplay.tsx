import { useState } from "react"
import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import { FolderOpen, Copy, Code, Check } from "lucide-react"
import { openDirectory, openInVSCode } from "@/services"
import { toast } from "sonner"

interface PathHoverDisplayProps {
  path: string           // 完整路径
  displayName?: string   // 显示名称（默认取路径最后一段）
  className?: string     // 外层容器样式
}

export function PathHoverDisplay({ path, displayName, className }: PathHoverDisplayProps) {
  const [copied, setCopied] = useState(false)

  const displayText = displayName || path.split(/[\\/]/).filter(Boolean).pop() || path

  const handleCopy = async () => {
    await navigator.clipboard.writeText(path)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const handleOpenDirectory = async () => {
    try {
      await openDirectory(path)
    } catch (error) {
      toast.error(String(error))
    }
  }

  const handleOpenVSCode = async () => {
    try {
      await openInVSCode(path)
    } catch (error) {
      toast.error(String(error))
    }
  }

  return (
    <div className={cn("relative group min-w-0", className)}>
      {/* 基础显示 */}
      <div className="flex items-center gap-1.5 cursor-default">
        <FolderOpen className="w-4 h-4 shrink-0 text-gray-400" />
        <span className="truncate text-sm text-gray-600">{displayText}</span>
      </div>

      {/* 悬浮层 */}
      <div className="absolute left-0 top-full mt-1 hidden group-hover:flex items-center gap-2 bg-white border border-gray-200 rounded-md px-3 py-1.5 shadow-md z-10 min-w-[200px]">
        <span className="text-xs text-gray-600 truncate max-w-[300px]" title={path}>{path}</span>
        <div className="flex items-center gap-1 shrink-0">
          {/* 复制 */}
          <Button
            variant="ghost"
            size="sm"
            onClick={handleCopy}
            className="p-0.5 h-auto"
            title="复制路径"
          >
            {copied ? (
              <Check className="w-3 h-3 text-green-500" />
            ) : (
              <Copy className="w-3 h-3" />
            )}
          </Button>
          {/* 打开目录 */}
          <Button
            variant="ghost"
            size="sm"
            onClick={handleOpenDirectory}
            className="p-0.5 h-auto"
            title="打开目录"
          >
            <FolderOpen className="w-3 h-3" />
          </Button>
          {/* 打开 VSCode */}
          <Button
            variant="ghost"
            size="sm"
            onClick={handleOpenVSCode}
            className="p-0.5 h-auto"
            title="在 VSCode 中打开"
          >
            <Code className="w-3 h-3" />
          </Button>
        </div>
      </div>
    </div>
  )
}
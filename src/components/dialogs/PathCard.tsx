import { useState, useRef, useEffect } from "react"
import { createPortal } from "react-dom"
import { cn } from "@/lib/utils"
import { Bookmark } from "lucide-react"
import type { FavoritePath } from "@/types"

interface PathCardProps {
  path: FavoritePath
  onPinToggle: () => void
  onDelete: () => void
  onSelect: () => void
}

export function PathCard({ path, onPinToggle, onDelete, onSelect }: PathCardProps) {
  const [showMenu, setShowMenu] = useState(false)
  const [menuPos, setMenuPos] = useState({ x: 0, y: 0 })
  const cardRef = useRef<HTMLDivElement>(null)

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault()
    e.stopPropagation()

    // 记录鼠标点击位置
    setMenuPos({ x: e.clientX, y: e.clientY })
    setShowMenu(true)
  }

  // 点击外部关闭
  useEffect(() => {
    if (!showMenu) return

    const handleGlobalClick = () => {
      setShowMenu(false)
    }

    // 延迟添加监听
    const timer = setTimeout(() => {
      document.addEventListener('click', handleGlobalClick)
    }, 0)

    return () => {
      clearTimeout(timer)
      document.removeEventListener('click', handleGlobalClick)
    }
  }, [showMenu])

  // ESC 关闭
  useEffect(() => {
    if (!showMenu) return

    const handleEsc = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setShowMenu(false)
      }
    }

    document.addEventListener('keydown', handleEsc)
    return () => document.removeEventListener('keydown', handleEsc)
  }, [showMenu])

  const handleDelete = () => {
    setShowMenu(false)
    onDelete()
  }

  const handleCopy = () => {
    setShowMenu(false)
    navigator.clipboard.writeText(path.path)
  }

  return (
    <>
      <div
        ref={cardRef}
        className={cn(
          "inline-flex items-center rounded overflow-hidden",
          path.pinned
            ? "border-2 border-violet-500 bg-violet-50"
            : "border border-gray-200 bg-white hover:bg-gray-50"
        )}
        onContextMenu={handleContextMenu}
      >
        {/* 书签按钮 */}
        <button
          onClick={(e) => {
            e.stopPropagation()
            setShowMenu(false)
            onPinToggle()
          }}
          className={cn(
            "p-1.5 border-r transition-colors",
            path.pinned
              ? "bg-violet-200 hover:bg-violet-300"
              : "bg-gray-50 hover:bg-violet-100"
          )}
          title={path.pinned ? "取消置顶" : "置顶"}
        >
          <Bookmark
            className={cn(
              "w-4 h-4",
              path.pinned
                ? "text-violet-600 fill-violet-600"
                : "text-gray-400"
            )}
          />
        </button>

        {/* 路径名称 */}
        <span
          onClick={(e) => {
            e.stopPropagation()
            setShowMenu(false)
            onSelect()
          }}
          className="px-2 py-1 text-xs hover:underline cursor-pointer"
        >
          {path.path}
        </span>
      </div>

      {/* 右键菜单 - Portal 到 body */}
      {showMenu && createPortal(
        <div
          className="fixed bg-white border rounded-lg shadow-xl py-1 z-[99999]"
          style={{
            left: menuPos.x,
            top: menuPos.y,
          }}
          onClick={(e) => e.stopPropagation()}
          onContextMenu={(e) => e.preventDefault()}
        >
          <button
            type="button"
            onClick={handleDelete}
            className="block w-full px-4 py-2 text-sm text-left hover:bg-red-50 hover:text-red-600"
          >
            删除此路径
          </button>
          <button
            type="button"
            onClick={handleCopy}
            className="block w-full px-4 py-2 text-sm text-left hover:bg-gray-100 text-gray-700"
          >
            复制完整路径
          </button>
        </div>,
        document.body
      )}
    </>
  )
}
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
  const menuRef = useRef<HTMLDivElement>(null)

  // 显示完整路径（用户要求）
  const displayPath = path.path

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault()
    e.stopPropagation()

    // 关闭其他菜单
    setShowMenu(false)

    // 使用 setTimeout 确保状态更新后再打开
    setTimeout(() => {
      let x = e.clientX
      let y = e.clientY

      // 边界检查
      const menuWidth = 140
      const menuHeight = 80
      if (x + menuWidth > window.innerWidth) {
        x = window.innerWidth - menuWidth - 10
      }
      if (y + menuHeight > window.innerHeight) {
        y = window.innerHeight - menuHeight - 10
      }

      setMenuPos({ x, y })
      setShowMenu(true)
    }, 0)
  }

  // 点击外部关闭菜单
  useEffect(() => {
    if (!showMenu) return

    const handleClick = (e: MouseEvent) => {
      // 如果点击在卡片或菜单内部，不关闭
      if (
        cardRef.current?.contains(e.target as Node) ||
        menuRef.current?.contains(e.target as Node)
      ) {
        return
      }
      setShowMenu(false)
    }

    // 使用 setTimeout 确保菜单已渲染后再添加监听
    const timer = setTimeout(() => {
      document.addEventListener("click", handleClick, true)
    }, 0)

    return () => {
      clearTimeout(timer)
      document.removeEventListener("click", handleClick, true)
    }
  }, [showMenu])

  const handleDelete = () => {
    setShowMenu(false)
    onDelete()
  }

  const handleCopyPath = () => {
    setShowMenu(false)
    navigator.clipboard.writeText(path.path)
  }

  return (
    <>
      <div
        ref={cardRef}
        className={cn(
          "inline-flex items-center rounded overflow-hidden cursor-pointer",
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
                : "text-gray-400 hover:text-violet-600"
            )}
          />
        </button>

        {/* 路径名称 - 显示完整路径 */}
        <span
          onClick={onSelect}
          className="px-3 py-1 text-xs hover:underline whitespace-nowrap"
        >
          {displayPath}
        </span>
      </div>

      {/* 右键菜单 - Portal 到 body */}
      {showMenu && createPortal(
        <div
          ref={menuRef}
          className="fixed bg-white border rounded-lg shadow-lg py-1 z-[9999]"
          style={{
            left: menuPos.x,
            top: menuPos.y,
            minWidth: "140px"
          }}
          onClick={(e) => e.stopPropagation()}
        >
          <button
            onClick={handleDelete}
            className="w-full px-3 py-2 text-sm text-left hover:bg-red-50 hover:text-red-600 transition-colors"
          >
            删除此路径
          </button>
          <button
            onClick={handleCopyPath}
            className="w-full px-3 py-2 text-sm text-left hover:bg-gray-100 text-gray-700 transition-colors"
          >
            复制完整路径
          </button>
        </div>,
        document.body
      )}
    </>
  )
}
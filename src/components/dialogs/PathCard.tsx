import { useState, useRef } from "react"
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

  // 显示完整路径
  const displayPath = path.path

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault()
    e.stopPropagation()

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
  }

  const handleDelete = () => {
    setShowMenu(false)
    onDelete()
  }

  const handleCopyPath = () => {
    setShowMenu(false)
    navigator.clipboard.writeText(path.path)
  }

  const closeMenu = () => {
    setShowMenu(false)
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

        {/* 路径名称 */}
        <span
          onClick={onSelect}
          className="px-3 py-1 text-xs hover:underline whitespace-nowrap"
        >
          {displayPath}
        </span>
      </div>

      {/* 右键菜单 - 使用 backdrop 确保能接收点击 */}
      {showMenu && createPortal(
        <>
          {/* 透明 backdrop - 拦截所有外部点击 */}
          <div
            className="fixed inset-0 z-[100]"
            onClick={closeMenu}
            onContextMenu={(e) => {
              e.preventDefault()
              closeMenu()
            }}
          />
          {/* 菜单内容 - 在 backdrop 之上 */}
          <div
            className="fixed bg-white border rounded-lg shadow-lg py-1 z-[101]"
            style={{
              left: menuPos.x,
              top: menuPos.y,
              minWidth: "140px"
            }}
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
          </div>
        </>,
        document.body
      )}
    </>
  )
}
import { useState, useRef, useEffect } from "react"
import { cn } from "@/lib/utils"
import { Bookmark } from "lucide-react"
import type { FavoritePath } from "@/types"

// 全局事件：关闭所有右键菜单
const CLOSE_ALL_MENUS_EVENT = 'close-all-context-menus'

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

  // 提取最后一级目录名
  const displayName = path.path.split(/[/\\]/).filter(Boolean).pop() || path.path

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault()
    // 先关闭所有其他菜单
    document.dispatchEvent(new CustomEvent(CLOSE_ALL_MENUS_EVENT))

    // 使用鼠标点击位置（clientX/clientY 是相对于视口的坐标）
    // fixed 定位的元素使用视口坐标是正确的
    let x = e.clientX
    let y = e.clientY

    // 确保菜单不超出视口右边界
    const menuWidth = 140
    if (x + menuWidth > window.innerWidth) {
      x = window.innerWidth - menuWidth - 10
    }

    // 确保菜单不超出视口底部
    const menuHeight = 80
    if (y + menuHeight > window.innerHeight) {
      y = window.innerHeight - menuHeight - 10
    }

    setMenuPos({ x, y })
    setShowMenu(true)
  }

  // 监听全局关闭事件
  useEffect(() => {
    const handleClose = () => setShowMenu(false)
    document.addEventListener(CLOSE_ALL_MENUS_EVENT, handleClose)
    return () => {
      document.removeEventListener(CLOSE_ALL_MENUS_EVENT, handleClose)
    }
  }, [])

  // 点击外部关闭
  useEffect(() => {
    if (showMenu) {
      const handleClickOutside = (e: MouseEvent) => {
        if (cardRef.current && !cardRef.current.contains(e.target as Node)) {
          setShowMenu(false)
        }
      }
      document.addEventListener("click", handleClickOutside)
      return () => {
        document.removeEventListener("click", handleClickOutside)
      }
    }
  }, [showMenu])

  return (
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
        className="px-3 py-1 text-xs hover:underline"
      >
        {displayName}
      </span>

      {/* 右键菜单 */}
      {showMenu && (
        <div
          className="fixed bg-white border rounded-lg shadow-lg py-1 z-50"
          style={{
            left: menuPos.x,
            top: menuPos.y,
            minWidth: "120px"
          }}
        >
          <button
            onClick={() => {
              setShowMenu(false)
              onDelete()
            }}
            className="w-full px-3 py-2 text-sm text-left hover:bg-red-50 hover:text-red-600 flex items-center gap-2"
          >
            删除此路径
          </button>
          <button
            onClick={() => {
              setShowMenu(false)
              navigator.clipboard.writeText(path.path)
            }}
            className="w-full px-3 py-2 text-sm text-left hover:bg-gray-50 text-gray-600 flex items-center gap-2"
          >
            复制完整路径
          </button>
        </div>
      )}
    </div>
  )
}
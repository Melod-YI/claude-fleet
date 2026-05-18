// src/components/common/EditableName.tsx

import { useState, useRef, useEffect } from 'react'
import { cn } from '@/lib/utils'
import { Pencil, X } from 'lucide-react'

interface EditableNameProps {
  name: string
  onSave: (newName: string) => Promise<void>
  className?: string
  onDoubleClick?: () => void
}

export function EditableName({ name, onSave, className, onDoubleClick }: EditableNameProps) {
  const [isEditing, setIsEditing] = useState(false)
  const [editValue, setEditValue] = useState(name)
  const [saving, setSaving] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (isEditing && inputRef.current) {
      inputRef.current.focus()
      inputRef.current.select()
    }
  }, [isEditing])

  const handleStartEdit = () => {
    setEditValue(name)
    setIsEditing(true)
  }

  const handleCancel = () => {
    setIsEditing(false)
    setEditValue(name)
  }

  const handleSave = async () => {
    if (saving) return

    const trimmedValue = editValue.trim()

    // 如果值为空或与原值相同，取消
    if (!trimmedValue || trimmedValue === name) {
      handleCancel()
      return
    }

    setSaving(true)
    try {
      await onSave(trimmedValue)
      setIsEditing(false)
    } catch (e) {
      console.error('保存名称失败:', e)
      setEditValue(name)
    } finally {
      setSaving(false)
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleSave()
    } else if (e.key === 'Escape') {
      handleCancel()
    }
  }

  const handleBlur = () => {
    handleSave()
  }

  if (isEditing) {
    return (
      <div className="flex items-center gap-1">
        <input
          ref={inputRef}
          type="text"
          value={editValue}
          onChange={(e) => setEditValue(e.target.value)}
          onKeyDown={handleKeyDown}
          onBlur={handleBlur}
          disabled={saving}
          className={cn(
            "px-2 py-1 text-sm border rounded focus:outline-none focus:ring-2 focus:ring-violet-500",
            saving && "opacity-50 cursor-not-allowed",
            className
          )}
        />
        <button
          onClick={handleCancel}
          className="p-1 hover:bg-gray-100 rounded"
          type="button"
        >
          <X className="w-4 h-4 text-gray-400" />
        </button>
      </div>
    )
  }

  return (
    <div
      className={cn("flex items-center gap-1 group cursor-pointer", className)}
      onDoubleClick={(e) => {
        e.stopPropagation()
        handleStartEdit()
        onDoubleClick?.()
      }}
    >
      <span className="truncate">{name}</span>
      <button
        onClick={(e) => {
          e.stopPropagation()
          handleStartEdit()
        }}
        className="p-1 opacity-0 group-hover:opacity-100 hover:bg-gray-100 rounded transition-opacity"
        type="button"
      >
        <Pencil className="w-3.5 h-3.5 text-gray-400" />
      </button>
    </div>
  )
}
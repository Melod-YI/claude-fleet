import { cn } from "@/lib/utils"
import type { SessionStatus } from "@/types"

interface StatusBadgeProps {
  status: SessionStatus
  className?: string
}

const statusConfig: Record<SessionStatus, { label: string; className: string; icon: string }> = {
  running: {
    label: "运行中",
    className: "bg-green-500 text-white",
    icon: "●",
  },
  waiting_input: {
    label: "等待输入",
    className: "bg-amber-500 text-white",
    icon: "⏳",
  },
  completed: {
    label: "已完成",
    className: "bg-gray-500 text-white",
    icon: "✓",
  },
  idle: {
    label: "空闲",
    className: "bg-gray-300 text-gray-600",
    icon: "○",
  },
}

export function StatusBadge({ status, className }: StatusBadgeProps) {
  const config = statusConfig[status]

  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium",
        config.className,
        className
      )}
    >
      <span>{config.icon}</span>
      <span>{config.label}</span>
    </span>
  )
}
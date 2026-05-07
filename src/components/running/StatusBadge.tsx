import { cn } from "@/lib/utils"
import type { SessionStatus } from "@/types"

interface StatusBadgeProps {
  status: SessionStatus
  className?: string
}

// 状态显示配置：busy 显示为运行中，idle/waiting 都显示为等待输入
const statusConfig: Record<SessionStatus, { label: string; className: string; icon: string }> = {
  busy: {
    label: "运行中",
    className: "bg-green-500 text-white",
    icon: "●",
  },
  idle: {
    label: "等待输入",
    className: "bg-amber-500 text-white",
    icon: "⏳",
  },
  waiting: {
    label: "等待输入",
    className: "bg-amber-500 text-white",
    icon: "⏳",
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
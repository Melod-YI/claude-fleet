import { ScrollArea } from "@/components/ui/scroll-area"
import type { ConversationMessage } from "@/types"
import { cn } from "@/lib/utils"

interface ConversationViewProps {
  messages: ConversationMessage[]
  loading?: boolean
}

export function ConversationView({ messages, loading }: ConversationViewProps) {
  if (loading) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        加载对话内容...
      </div>
    )
  }

  if (messages.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        没有对话记录
      </div>
    )
  }

  return (
    <ScrollArea className="h-full">
      <div className="flex flex-col gap-4 p-4 min-w-0">
        {messages.map((message) => (
          <div
            key={message.id}
            className={cn(
              "flex gap-3 min-w-0",
              message.role === "user" ? "flex-row" : "flex-row"
            )}
          >
            {/* 头像 */}
            <div
              className={cn(
                "w-9 h-9 rounded-full flex items-center justify-center text-white text-sm font-medium shrink-0",
                message.role === "user" ? "bg-violet-600" : "bg-green-600"
              )}
            >
              {message.role === "user" ? "你" : "C"}
            </div>

            {/* 消息内容 */}
            <div className="flex-1 min-w-0">
              <div
                className={cn(
                  "rounded-lg p-3 min-w-0",
                  message.role === "user"
                    ? "bg-gray-100"
                    : "bg-green-50"
                )}
              >
                <p className="text-sm whitespace-pre-wrap break-words overflow-wrap-anywhere">{message.content}</p>
              </div>
              <span className="text-xs text-gray-500 mt-1">
                {new Date(message.timestamp).toLocaleString("zh-CN")}
              </span>
            </div>
          </div>
        ))}
      </div>
    </ScrollArea>
  )
}
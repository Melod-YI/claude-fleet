export type MessageRole = 'user' | 'assistant'

export interface ConversationMessage {
  id: string
  role: MessageRole
  content: string
  timestamp: string  // ISO datetime
}

export interface Conversation {
  sessionId: string
  messages: ConversationMessage[]
  totalMessages: number
}
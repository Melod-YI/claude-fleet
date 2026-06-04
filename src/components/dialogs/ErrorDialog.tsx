import { useState } from "react"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Copy, Check } from "lucide-react"

interface ErrorDialogProps {
  open: boolean
  onClose: () => void
  title: string
  message: string
}

export function ErrorDialog({ open, onClose, title, message }: ErrorDialogProps) {
  const [copied, setCopied] = useState(false)

  const handleCopy = async () => {
    await navigator.clipboard.writeText(message)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[560px]">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>

        <pre className="max-h-[45vh] overflow-auto whitespace-pre-wrap break-words rounded-md border bg-muted/40 p-3 text-sm select-text">
          {message}
        </pre>

        <DialogFooter>
          <Button variant="outline" onClick={handleCopy}>
            {copied ? (
              <>
                <Check className="mr-2 h-4 w-4" />
                已复制
              </>
            ) : (
              <>
                <Copy className="mr-2 h-4 w-4" />
                复制错误
              </>
            )}
          </Button>
          <Button onClick={onClose}>关闭</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

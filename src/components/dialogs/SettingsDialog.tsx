import { useSettingsStore } from '@/stores/settingsStore'
import type { TerminalType } from '@/types'
import { Switch } from '@/components/ui/switch'
import { Label } from '@/components/ui/label'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Button } from '@/components/ui/button'

interface SettingsDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

const TERMINAL_OPTIONS: { value: TerminalType; label: string }[] = [
  { value: 'wezterm', label: 'WezTerm' },
  { value: 'cmd', label: '命令提示符' },
  { value: 'powershell', label: 'PowerShell' },
]

export function SettingsDialog({ open, onOpenChange }: SettingsDialogProps) {
  const {
    terminalType,
    setTerminalType,
    notificationSound,
    setNotificationSound,
    notificationDesktop,
    setNotificationDesktop,
  } = useSettingsStore()

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[400px]">
        <DialogHeader>
          <DialogTitle>设置</DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* 终端选择 */}
          <div className="space-y-2">
            <label className="text-sm font-medium">默认终端</label>
            <Select
              value={terminalType}
              onValueChange={(value) => setTerminalType(value as TerminalType)}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {TERMINAL_OPTIONS.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    {option.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">
              选择恢复 session 时使用的终端
            </p>
          </div>

          {/* 通知设置 */}
          <div className="space-y-3 border-t pt-4">
            <label className="text-sm font-medium">通知设置</label>

            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label htmlFor="notification-sound">提示音</Label>
                <p className="text-xs text-muted-foreground">
                  Session 进入等待状态时播放提示音
                </p>
              </div>
              <Switch
                id="notification-sound"
                checked={notificationSound}
                onCheckedChange={setNotificationSound}
              />
            </div>

            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label htmlFor="notification-desktop">桌面通知</Label>
                <p className="text-xs text-muted-foreground">
                  发送 Windows 系统通知
                </p>
              </div>
              <Switch
                id="notification-desktop"
                checked={notificationDesktop}
                onCheckedChange={setNotificationDesktop}
              />
            </div>
          </div>
        </div>

        <div className="flex justify-end">
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            关闭
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
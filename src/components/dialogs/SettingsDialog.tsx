import { useEffect, useState } from 'react'
import { useSettingsStore } from '@/stores/settingsStore'
import type { TerminalType, SoundInfo } from '@/types'
import { Switch } from '@/components/ui/switch'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
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
import { getAvailableSounds, previewSound, BUILTIN_DEFAULT_ID } from '@/services/soundService'
import { Volume2, AlertCircle } from 'lucide-react'

interface SettingsDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

const TERMINAL_OPTIONS: { value: TerminalType; label: string }[] = [
  { value: 'wezterm', label: 'WezTerm' },
  { value: 'cmd', label: '命令提示符' },
  { value: 'powershell', label: 'PowerShell' },
  { value: 'powershell7', label: 'PowerShell 7' },
]

export function SettingsDialog({ open, onOpenChange }: SettingsDialogProps) {
  const {
    terminalType,
    setTerminalType,
    launchSettings,
    setLaunchSettings,
    notificationSound,
    setNotificationSound,
    notificationDesktop,
    setNotificationDesktop,
    notificationSoundFile,
    setNotificationSoundFile,
  } = useSettingsStore()

  const [sounds, setSounds] = useState<SoundInfo[]>([])
  const [loadingSounds, setLoadingSounds] = useState(true)
  const [soundLoadError, setSoundLoadError] = useState<string | null>(null)
  const [soundMissing, setSoundMissing] = useState(false)
  const [previewing, setPreviewing] = useState(false)
  const [activeTab, setActiveTab] = useState<'launch' | 'notifications'>('launch')
  const wrapper = launchSettings.wrapper ?? {
    enabled: false,
    executable: 'ccglass',
    argsBeforeAgent: [],
  }

  const updateLaunchSettings = (next: Partial<typeof launchSettings>) => {
    setLaunchSettings({
      ...launchSettings,
      ...next,
    })
  }

  const updateWrapper = (next: Partial<typeof wrapper>) => {
    setLaunchSettings({
      ...launchSettings,
      wrapper: {
        ...wrapper,
        ...next,
      },
    })
  }

  // 加载可用音频列表
  useEffect(() => {
    if (open) {
      setLoadingSounds(true)
      setSoundLoadError(null)
      setSoundMissing(false)

      getAvailableSounds()
        .then((soundList) => {
          setSounds(soundList)

          // 检查当前选中的音频是否有效
          // 空字符串或 BUILTIN_DEFAULT_ID 都视为内置默认（有效）
          if (notificationSoundFile && notificationSoundFile !== BUILTIN_DEFAULT_ID) {
            const isValid = soundList.some(s => s.filename === notificationSoundFile)
            if (!isValid) {
              setSoundMissing(true)
              // 重置为内置默认
              setNotificationSoundFile(BUILTIN_DEFAULT_ID)
            }
          }
        })
        .catch((err) => {
          console.error('[SettingsDialog] 加载音频列表失败:', err)
          setSoundLoadError('加载音频列表失败')
        })
        .finally(() => setLoadingSounds(false))
    }
  }, [open, notificationSoundFile, setNotificationSoundFile])

  // 试听音频
  const handlePreview = async () => {
    if (previewing || sounds.length === 0) return

    // 空字符串视为内置默认
    const soundToPlay = notificationSoundFile || BUILTIN_DEFAULT_ID

    setPreviewing(true)
    try {
      await previewSound(soundToPlay)
    } catch (err) {
      console.error('[SettingsDialog] 试听失败:', err)
    } finally {
      setTimeout(() => setPreviewing(false), 500)
    }
  }

  // 获取显示名称
  const getDisplayName = (sound: SoundInfo) => {
    if (sound.isBuiltin) return `${sound.name}（系统默认）`
    return sound.name
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[720px] max-h-[86vh] overflow-hidden">
        <DialogHeader>
          <DialogTitle>设置</DialogTitle>
        </DialogHeader>

        <div className="flex rounded-md border bg-muted/40 p-1">
          <button
            type="button"
            className={`flex-1 rounded-sm px-3 py-1.5 text-sm transition-colors ${
              activeTab === 'launch' ? 'bg-background shadow-sm' : 'text-muted-foreground'
            }`}
            onClick={() => setActiveTab('launch')}
          >
            启动
          </button>
          <button
            type="button"
            className={`flex-1 rounded-sm px-3 py-1.5 text-sm transition-colors ${
              activeTab === 'notifications' ? 'bg-background shadow-sm' : 'text-muted-foreground'
            }`}
            onClick={() => setActiveTab('notifications')}
          >
            通知
          </button>
        </div>

        <div className="max-h-[61vh] overflow-y-auto px-1 py-4">
          {activeTab === 'launch' && (
            <div className="space-y-4">
              <div className="space-y-2">
                <label className="text-sm font-medium">默认终端</label>
                <Select
                  value={terminalType}
                  onValueChange={(value) => setTerminalType(value as TerminalType)}
                >
                  <SelectTrigger className="focus:ring-inset">
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
                  选择新建和恢复 session 时使用的终端
                </p>
              </div>

              <div className="space-y-2">
                <Label htmlFor="claude-args">Claude 参数</Label>
                <Input
                  id="claude-args"
                  className="focus-visible:ring-inset"
                  value={joinArgs(launchSettings.claudeArgs)}
                  onChange={(event) => updateLaunchSettings({ claudeArgs: splitArgs(event.target.value) })}
                  placeholder="--permission-mode bypassPermissions"
                />
                <p className="text-xs text-muted-foreground">
                  新建和恢复时都会附加这些参数；留空则不附加默认权限参数
                </p>
              </div>

              <div className={`flex items-center justify-between rounded-md border p-3 ${terminalType === 'wezterm' ? 'opacity-60' : ''}`}>
                <div className="space-y-0.5">
                  <Label htmlFor="ccglass-enabled">启用 ccglass</Label>
                  <p className="text-xs text-muted-foreground">
                    启用后使用 ccglass 作为主入口
                  </p>
                  {terminalType === 'wezterm' && (
                    <p className="text-xs text-amber-600">
                      WezTerm 暂不支持 ccglass，已自动禁用
                    </p>
                  )}
                </div>
                <Switch
                  id="ccglass-enabled"
                  checked={terminalType === 'wezterm' ? false : wrapper.enabled}
                  disabled={terminalType === 'wezterm'}
                  onCheckedChange={(enabled) => updateWrapper({ enabled })}
                />
              </div>

              {wrapper.enabled && terminalType !== 'wezterm' && (
                <div className="space-y-3 rounded-md border bg-muted/20 p-3">
                  <div className="space-y-2">
                    <Label htmlFor="ccglass-executable">ccglass 可执行文件</Label>
                    <Input
                      id="ccglass-executable"
                      value={wrapper.executable}
                      onChange={(event) => updateWrapper({ executable: event.target.value })}
                      placeholder="ccglass"
                    />
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="ccglass-args">ccglass 参数</Label>
                    <Input
                      id="ccglass-args"
                      value={joinArgs(wrapper.argsBeforeAgent)}
                      onChange={(event) => updateWrapper({ argsBeforeAgent: splitArgs(event.target.value) })}
                      placeholder="可选，追加在 claude 之前"
                    />
                  </div>
                </div>
              )}
            </div>
          )}

          {activeTab === 'notifications' && (
            <div className="space-y-3">
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

              {notificationSound && (
                <div className="space-y-2 rounded-md border bg-muted/20 p-3">
                  <label className="text-sm">音频文件</label>

                  {soundLoadError && (
                    <div className="flex items-center gap-2 text-xs text-amber-600">
                      <AlertCircle className="h-3 w-3" />
                      <span>{soundLoadError}，将使用默认音频</span>
                    </div>
                  )}

                  {soundMissing && (
                    <div className="flex items-center gap-2 text-xs text-amber-600">
                      <AlertCircle className="h-3 w-3" />
                      <span>之前选择的音频已不存在，已重置为默认</span>
                    </div>
                  )}

                  <div className="flex items-center gap-2">
                    <Select
                      value={notificationSoundFile || BUILTIN_DEFAULT_ID}
                      onValueChange={setNotificationSoundFile}
                      disabled={loadingSounds || sounds.length === 0}
                    >
                      <SelectTrigger className="flex-1">
                        <SelectValue placeholder={loadingSounds ? "加载中..." : "选择音频"} />
                      </SelectTrigger>
                      <SelectContent>
                        {sounds.map((sound) => (
                          <SelectItem key={sound.filename} value={sound.filename}>
                            {getDisplayName(sound)}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>

                    <Button
                      variant="outline"
                      size="icon"
                      onClick={handlePreview}
                      disabled={loadingSounds || previewing || sounds.length === 0}
                      title="试听音频"
                    >
                      <Volume2 className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              )}

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
          )}
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

function splitArgs(value: string): string[] {
  const matches = value.match(/"[^"]*"|'[^']*'|\S+/g) ?? []
  return matches.map((arg) => {
    if (
      (arg.startsWith('"') && arg.endsWith('"')) ||
      (arg.startsWith("'") && arg.endsWith("'"))
    ) {
      return arg.slice(1, -1)
    }
    return arg
  })
}

function joinArgs(args: string[]): string {
  return args.map((arg) => {
    if (!arg) return '""'
    if (!/\s|["']/.test(arg)) return arg
    return `"${arg.replace(/"/g, '\\"')}"`
  }).join(' ')
}
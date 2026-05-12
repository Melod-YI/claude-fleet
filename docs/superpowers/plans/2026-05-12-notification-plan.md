# 通知功能实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现可靠的通知功能，包含内嵌提示音和 Windows Toast 通知

**Architecture:** 使用 tauri-plugin-notification 实现 Windows 系统通知，前端内嵌 base64 提示音，设置 UI 提供独立开关

**Tech Stack:** tauri-plugin-notification (Rust), @tauri-apps/plugin-notification (前端), HTMLAudioElement (提示音)

---

## 文件结构

```
src-tauri/
├── Cargo.toml                          # 添加 notification 插件依赖
├── src/
│   └── lib.rs                          # 初始化 notification 插件
└── src/commands/
    └── session.rs                       # 移除旧的 send_notification 命令

src/
├── services/
│   └── notificationService.ts          # 重写：base64 提示音 + Tauri 通知
├── components/
│   └── dialogs/
│       └── SettingsDialog.tsx          # 添加通知设置 UI
└── components/ui/
    └── switch.tsx                       # 新增 Switch 组件（已有 @radix-ui/react-switch）

public/sounds/                          # 保持空，无需文件
```

---

## Task 1: 添加 Tauri 通知插件依赖

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs`
- Modify: `package.json`

### Step 1: 添加 Rust 依赖

在 `src-tauri/Cargo.toml` 的 `[dependencies]` 节添加：

```toml
tauri-plugin-notification = "2"
```

### Step 2: 初始化 Rust 插件

在 `src-tauri/src/lib.rs` 中：

1. 在 `commands` 模块导入处添加：
```rust
mod commands;
// ... existing imports

// 添加 notification 插件
#[cfg(windows)]
use tauri_plugin_notification::NotificationExt;
```

2. 在 `builder()` 的 `.setup()` 中初始化：
```rust
.plugin(tauri_plugin_notification::init())
```

3. 移除 `send_notification` 命令的导入（commands/mod.rs 或 lib.rs 中的）
4. 删除 `src-tauri/src/commands/session.rs` 中的 `send_notification` 函数

### Step 3: 添加前端依赖

在 `package.json` 的 `dependencies` 中添加：

```json
"@tauri-apps/plugin-notification": "^2.0.0"
```

运行 `npm install` 安装依赖。

---

## Task 2: 重写通知服务（内嵌提示音 + Tauri 通知）

**Files:**
- Modify: `src/services/notificationService.ts`

### Step 1: 准备 base64 提示音

生成一个短提示音的 base64 编码。使用以下简单的"叮"声（约 1 秒，8kHz，单声道）：

```typescript
// 简短的提示音 base64（实际项目中需要替换为真正的音频数据）
// 这里使用一个占位符，实际音频需要在实现时生成
const NOTIFICATION_SOUND_BASE64 = "data:audio/mp3;base64,...";
```

**注意：** 实际音频需要通过工具生成或从公共资源获取。可使用 Python 脚本生成：
```python
# 生成提示音（需要安装 pydub 和 scipy）
import scipy.io.wavfile as wav
import numpy as np
import base64

sample_rate = 8000
duration = 0.5  # 0.5秒
frequency = 880  # A5 音调

t = np.linspace(0, duration, int(sample_rate * duration), False)
wave = np.sin(2 * np.pi * frequency * t) * 0.3
wave = (wave * 32767).astype(np.int16)

import io
import soundfile as sf
buffer = io.BytesIO()
sf.write(buffer, wave, sample_rate, format='WAV')
audio_bytes = buffer.getvalue()
base64_audio = base64.b64encode(audio_bytes).decode('utf-8')
print(base64_audio)
```

### Step 2: 重写 notificationService.ts

完全重写文件：

```typescript
import { isPermissionGranted, requestPermission, sendNotification as tauriSendNotification } from '@tauri-apps/plugin-notification'

// 内嵌 base64 提示音（简短的"叮"声）
const NOTIFICATION_SOUND_BASE64 = "data:audio/wav;base64,UklGRiQAAABXQVZFZm10IBAAAAABAAEARKwAAIhYAQACABAAZGF0YQAAAAA="

// 预加载提示音对象
let notificationSound: HTMLAudioElement | null = null

export interface NotificationOptions {
  title: string
  body: string
  sessionId?: string
  sound?: boolean
}

/**
 * 初始化音频对象
 */
function initAudio(): void {
  if (!notificationSound) {
    try {
      notificationSound = new Audio(NOTIFICATION_SOUND_BASE64)
      notificationSound.volume = 0.5
      notificationSound.preload = 'auto'
    } catch (e) {
      console.error('[notificationService] 初始化音频失败:', e)
    }
  }
}

/**
 * 播放提示音
 */
export function playNotificationSound(): void {
  initAudio()
  if (notificationSound) {
    notificationSound.currentTime = 0
    notificationSound.play().catch((e) => {
      console.warn('[notificationService] 播放提示音失败:', e)
    })
  }
}

/**
 * 发送桌面通知
 */
export async function sendDesktopNotification(options: NotificationOptions): Promise<void> {
  try {
    // 检查权限
    let permissionGranted = await isPermissionGranted()
    if (!permissionGranted) {
      const permission = await requestPermission()
      permissionGranted = permission === 'granted'
    }

    if (permissionGranted) {
      await tauriSendNotification({
        title: options.title,
        body: options.body,
      })
      console.log('[notificationService] 桌面通知已发送:', options.title)
    } else {
      console.warn('[notificationService] 通知权限未授权')
    }
  } catch (e) {
    console.error('[notificationService] 发送桌面通知失败:', e)
  }

  // 根据设置决定是否播放提示音
  if (options.sound) {
    playNotificationSound()
  }
}

/**
 * 初始化通知服务
 */
export async function initNotificationService(): Promise<void> {
  initAudio()

  // 检查当前权限状态
  const permissionGranted = await isPermissionGranted()
  if (!permissionGranted) {
    console.log('[notificationService] 通知权限未授权，将在需要时请求')
  } else {
    console.log('[notificationService] 通知权限已授权')
  }
}
```

---

## Task 3: 添加通知设置 UI

**Files:**
- Modify: `src/components/dialogs/SettingsDialog.tsx`
- Modify: `src/stores/settingsStore.ts` (验证字段存在)

### Step 1: 检查设置字段

确认 `src/stores/settingsStore.ts` 中已有：
- `notificationSound: boolean` (默认 true)
- `notificationDesktop: boolean` (默认 true)
- `setNotificationSound: (enabled: boolean) => void`
- `setNotificationDesktop: (enabled: boolean) => void`

从之前阅读的代码看，这些字段已存在，无需修改 store。

### Step 2: 添加 Switch 组件

检查 `src/components/ui/switch.tsx` 是否存在。如果不存在，从 shadcn/ui 添加：

```typescript
// src/components/ui/switch.tsx
import * as React from "react"
import * as SwitchPrimitives from "@radix-ui/react-switch"

import { cn } from "@/lib/utils"

const Switch = React.forwardRef<
  React.ElementRef<typeof SwitchPrimitives.Root>,
  React.ComponentPropsWithoutRef<typeof SwitchPrimitives.Root>
>(({ className, ...props }, ref) => (
  <SwitchPrimitives.Root
    className={cn(
      "peer inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-slate-950 focus-visible:ring-offset-2 focus-visible:ring-offset-white disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-slate-900 data-[state=unchecked]:bg-slate-200",
      className
    )}
    {...props}
    ref={ref}
  >
    <SwitchPrimitives.Thumb
      className={cn(
        "pointer-events-none block h-4 w-4 rounded-full bg-white shadow-lg ring-0 transition-transform data-[state=checked]:translate-x-4 data-[state=unchecked]:translate-x-0"
      )}
    />
  </SwitchPrimitives.Root>
))
Switch.displayName = SwitchPrimitives.Root.displayName

export { Switch }
```

### Step 3: 更新 SettingsDialog

修改 `src/components/dialogs/SettingsDialog.tsx`：

```typescript
import { useSettingsStore } from '@/stores/settingsStore'
import type { TerminalType } from '@/types'
import { Switch } from '@/components/ui/switch'
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
import { Label } from '@/components/ui/label'

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
                <Label htmlFor="notification-sound" className="text-sm">
                  提示音
                </Label>
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
                <Label htmlFor="notification-desktop" className="text-sm">
                  桌面通知
                </Label>
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
```

### Step 4: 确保 Label 组件存在

检查 `src/components/ui/label.tsx` 是否存在。如果不存在，添加：

```typescript
// src/components/ui/label.tsx
import * as React from "react"
import * as LabelPrimitive from "@radix-ui/react-label"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"

const labelVariants = cva(
  "text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
)

const Label = React.forwardRef<
  React.ElementRef<typeof LabelPrimitive.Root>,
  React.ComponentPropsWithoutRef<typeof LabelPrimitive.Root> &
    VariantProps<typeof labelVariants>
>(({ className, ...props }, ref) => (
  <LabelPrimitive.Root
    ref={ref}
    className={cn(labelVariants(), className)}
    {...props}
  />
))
Label.displayName = LabelPrimitive.Root.displayName

export { Label }
```

---

## Task 4: 验证和测试

### Step 1: 构建前端

```bash
cd C:\workspace\claude-fleet-sp
npm run build
```

检查是否有 TypeScript 错误。

### Step 2: 构建 Tauri 应用

```bash
npm run tauri build
```

检查 Rust 编译是否通过。

### Step 3: 手动测试

1. 启动应用（开发模式 `npm run tauri dev`）
2. 打开设置对话框，确认通知开关显示正常
3. 切换通知开关，验证设置能正确保存
4. 让一个 Claude Code session 进入等待状态，验证：
   - 提示音是否播放
   - Windows 通知是否弹出

### Step 4: 验证日志

检查日志输出：
- `[notificationService] 初始化音频成功` 或 `初始化音频失败`
- `[notificationService] 桌面通知已发送`
- `[notificationService] 播放提示音失败`

---

## 验收标准

1. ✅ 前端 TypeScript 编译通过
2. ✅ Rust 编译通过，notification 插件正确初始化
3. ✅ 设置对话框显示通知开关（提示音、桌面通知）
4. ✅ 两个开关可独立控制，设置能正确保存和加载
5. ✅ session 进入等待状态时，提示音能正常播放（或明确报错）
6. ✅ session 进入等待状态时，Windows Toast 通知能正常弹出
7. ✅ 关闭开关后，对应通知不再触发

---

## 注意事项

1. **音频文件**：当前 base64 是占位符，需要替换为真正的音频数据。可以使用 Python 脚本生成一个简单的"叮"声，或从公共资源获取。
2. **权限请求**：Windows 通知需要在首次使用时请求权限，tauri-plugin-notification 会自动处理。
3. **日志记录**：notificationService.ts 已添加详细的日志，便于调试和问题定位。
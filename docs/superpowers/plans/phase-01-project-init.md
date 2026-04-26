# Phase 1: 项目初始化

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 创建 Tauri + React + TypeScript 项目骨架，配置 Tailwind CSS 和 shadcn/ui

**Architecture:** 使用 Tauri 2.0 作为桌面框架，Vite 作为前端构建工具，React + TypeScript 作为 UI 层，Tailwind CSS + shadcn/ui 作为样式系统

**Tech Stack:** Tauri 2.0, React 18, TypeScript 5, Vite 5, Tailwind CSS 3, shadcn/ui, Zustand

---

## Task 1.1: 创建 Tauri 项目

**Files:**
- Create: `package.json`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`

- [ ] **Step 1: 安装 Tauri CLI 和创建项目**

```bash
# 安装 Tauri CLI
npm install -g @anthropic-ai/tauri-cli

# 或者使用 cargo 安装
cargo install tauri-cli

# 创建项目（在当前目录）
cd C:\workspace\claude-fleet-sp
npm create tauri-app@latest . -- --template react-ts
```

Expected: 项目结构创建成功

- [ ] **Step 2: 配置 Tauri 基本信息**

编辑 `src-tauri/tauri.conf.json`：

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Claude Fleet",
  "version": "0.1.0",
  "identifier": "com.claude-fleet.app",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:5173",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "windows": [
      {
        "title": "Claude Fleet",
        "width": 1200,
        "height": 800,
        "resizable": true,
        "minWidth": 800,
        "minHeight": 600
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

- [ ] **Step 3: 配置 Cargo.toml 依赖**

编辑 `src-tauri/Cargo.toml`：

```toml
[package]
name = "claude-fleet"
version = "0.1.0"
edition = "2021"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["notification-all", "shell-open"] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dirs = "5"

[profile.release]
strip = true
lto = true
```

- [ ] **Step 4: 创建基础 Rust 入口**

编辑 `src-tauri/src/main.rs`：

```rust
// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "console")]

fn main() {
    tauri_build::build()
}
```

编辑 `src-tauri/src/lib.rs`：

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: 验证项目创建**

```bash
npm run tauri dev
```

Expected: 应用窗口打开，显示默认 React 页面

- [ ] **Step 6: Commit**

```bash
git add .
git commit -m "feat: 初始化 Tauri + React 项目结构"
```

---

## Task 1.2: 配置 Tailwind CSS

**Files:**
- Create: `tailwind.config.js`
- Create: `postcss.config.js`
- Modify: `src/index.css`
- Modify: `package.json`

- [ ] **Step 1: 安装 Tailwind CSS**

```bash
npm install -D tailwindcss postcss autoprefixer
npx tailwindcss init -p
```

Expected: `tailwind.config.js` 和 `postcss.config.js` 创建成功

- [ ] **Step 2: 配置 Tailwind**

编辑 `tailwind.config.js`：

```javascript
/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ["class"],
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        border: "hsl(var(--border))",
        input: "hsl(var(--input))",
        ring: "hsl(var(--ring))",
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        primary: {
          DEFAULT: "hsl(var(--primary))",
          foreground: "hsl(var(--primary-foreground))",
        },
        secondary: {
          DEFAULT: "hsl(var(--secondary))",
          foreground: "hsl(var(--secondary-foreground))",
        },
        destructive: {
          DEFAULT: "hsl(var(--destructive))",
          foreground: "hsl(var(--destructive-foreground))",
        },
        muted: {
          DEFAULT: "hsl(var(--muted))",
          foreground: "hsl(var(--muted-foreground))",
        },
        accent: {
          DEFAULT: "hsl(var(--accent))",
          foreground: "hsl(var(--accent-foreground))",
        },
        popover: {
          DEFAULT: "hsl(var(--popover))",
          foreground: "hsl(var(--popover-foreground))",
        },
        card: {
          DEFAULT: "hsl(var(--card))",
          foreground: "hsl(var(--card-foreground))",
        },
      },
      borderRadius: {
        lg: "var(--radius)",
        md: "calc(var(--radius) - 2px)",
        sm: "calc(var(--radius) - 4px)",
      },
      keyframes: {
        "accordion-down": {
          from: { height: "0" },
          to: { height: "var(--radix-accordion-content-height)" },
        },
        "accordion-up": {
          from: { height: "var(--radix-accordion-content-height)" },
          to: { height: "0" },
        },
      },
      animation: {
        "accordion-down": "accordion-down 0.2s ease-out",
        "accordion-up": "accordion-up 0.2s ease-out",
      },
    },
  },
  plugins: [require("tailwindcss-animate")],
}
```

- [ ] **Step 3: 配置全局样式**

编辑 `src/index.css`：

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  :root {
    --background: 0 0% 100%;
    --foreground: 222.2 84% 4.9%;
    --card: 0 0% 100%;
    --card-foreground: 222.2 84% 4.9%;
    --popover: 0 0% 100%;
    --popover-foreground: 222.2 84% 4.9%;
    --primary: 262.1 83.3% 58.6%;
    --primary-foreground: 210 20% 98%;
    --secondary: 210 40% 96.1%;
    --secondary-foreground: 222.2 47.4% 11.2%;
    --muted: 210 40% 96.1%;
    --muted-foreground: 215.4 16.3% 46.9%;
    --accent: 210 40% 96.1%;
    --accent-foreground: 222.2 47.4% 11.2%;
    --destructive: 0 84.2% 60.2%;
    --destructive-foreground: 210 20% 98%;
    --border: 214.3 31.8% 91.4%;
    --input: 214.3 31.8% 91.4%;
    --ring: 262.1 83.3% 58.6%;
    --radius: 0.5rem;
  }

  .dark {
    --background: 222.2 84% 4.9%;
    --foreground: 210 40% 98%;
    --card: 222.2 84% 4.9%;
    --card-foreground: 210 40% 98%;
    --popover: 222.2 84% 4.9%;
    --popover-foreground: 210 40% 98%;
    --primary: 263.4 70% 50.4%;
    --primary-foreground: 210 20% 98%;
    --secondary: 217.2 32.6% 17.5%;
    --secondary-foreground: 210 40% 98%;
    --muted: 217.2 32.6% 17.5%;
    --muted-foreground: 215 20.2% 65.1%;
    --accent: 217.2 32.6% 17.5%;
    --accent-foreground: 210 40% 98%;
    --destructive: 0 62.8% 30.6%;
    --destructive-foreground: 210 40% 98%;
    --border: 217.2 32.6% 17.5%;
    --input: 217.2 32.6% 17.5%;
    --ring: 263.4 70% 50.4%;
  }
}

@layer base {
  * {
    @apply border-border;
  }
  body {
    @apply bg-background text-foreground;
  }
}
```

- [ ] **Step 4: 安装 tailwindcss-animate**

```bash
npm install -D tailwindcss-animate
```

- [ ] **Step 5: 验证 Tailwind 配置**

```bash
npm run tauri dev
```

Expected: 应用运行，样式正常加载

- [ ] **Step 6: Commit**

```bash
git add .
git commit -m "feat: 配置 Tailwind CSS 和全局样式"
```

---

## Task 1.3: 安装和配置 shadcn/ui

**Files:**
- Create: `src/components/ui/` 目录及多个组件
- Create: `src/lib/utils.ts`
- Modify: `package.json`
- Modify: `tsconfig.json`

- [ ] **Step 1: 初始化 shadcn/ui**

```bash
npx shadcn@latest init
```

选择配置：
- Style: Default
- Base color: Violet
- CSS variables: Yes

Expected: `src/lib/utils.ts` 和 `components.json` 创建

- [ ] **Step 2: 安装需要的组件**

```bash
npx shadcn@latest add button
npx shadcn@latest add input
npx shadcn@latest add dialog
npx shadcn@latest add scroll-area
npx shadcn@latest add badge
npx shadcn@latest add toggle
npx shadcn@latest add select
npx shadcn@latest add dropdown-menu
npx shadcn@latest add separator
```

Expected: `src/components/ui/` 目录下创建各组件文件

- [ ] **Step 3: 创建自定义 cn 工具函数**

编辑 `src/lib/utils.ts`（如果 shadcn 已创建则检查内容）：

```typescript
import { type ClassValue, clsx } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}
```

- [ ] **Step 4: 安装额外依赖**

```bash
npm install clsx tailwind-merge class-variance-authority lucide-react
```

- [ ] **Step 5: 验证组件可用**

在 `src/App.tsx` 中测试一个 Button：

```typescript
import { Button } from "@/components/ui/button"

function App() {
  return (
    <div className="p-4">
      <Button>Test Button</Button>
    </div>
  )
}

export default App
```

```bash
npm run tauri dev
```

Expected: Button 正常显示

- [ ] **Step 6: Commit**

```bash
git add .
git commit -m "feat: 安装 shadcn/ui 组件库"
```

---

## Task 1.4: 安装 Zustand 状态管理

**Files:**
- Create: `src/stores/` 目录
- Create: `src/stores/index.ts`
- Modify: `package.json`

- [ ] **Step 1: 安装 Zustand**

```bash
npm install zustand
```

- [ ] **Step 2: 创建 stores 目录结构**

```bash
mkdir -p src/stores
```

- [ ] **Step 3: 创建 store 入口文件**

创建 `src/stores/index.ts`：

```typescript
// Store exports will be added here as they are created
export {}
```

- [ ] **Step 4: Commit**

```bash
git add .
git commit -m "feat: 安装 Zustand 状态管理"
```

---

## Task 1.5: 创建 TypeScript 类型定义

**Files:**
- Create: `src/types/session.ts`
- Create: `src/types/conversation.ts`
- Create: `src/types/settings.ts`
- Create: `src/types/index.ts`

- [ ] **Step 1: 创建 types 目录**

```bash
mkdir -p src/types
```

- [ ] **Step 2: 定义 Session 类型**

创建 `src/types/session.ts`：

```typescript
export type SessionStatus = 'running' | 'waiting_input' | 'completed' | 'idle'

export interface ClaudeSession {
  id: string
  name: string
  workingDirectory: string
  status: SessionStatus
  createdAt: string  // ISO datetime
  lastActivityAt: string  // ISO datetime
  conversationCount: number
  isFavorite: boolean
  terminalWindowId?: string  // Windows Terminal 窗口标识
  processId?: number  // Claude 进程 ID
}

export interface SessionFilter {
  searchQuery?: string
  showFavoritesOnly: boolean
  timeRange?: '3d' | '7d' | '30d' | 'all'
  status?: SessionStatus
}

export interface SessionCreateOptions {
  workingDirectory: string
  name?: string
  addToFavorites: boolean
}
```

- [ ] **Step 3: 定义 Conversation 类型**

创建 `src/types/conversation.ts`：

```typescript
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
```

- [ ] **Step 4: 定义 Settings 类型**

创建 `src/types/settings.ts`：

```typescript
export interface FavoritePaths {
  paths: string[]
}

export interface AppSettings {
  favoritePaths: FavoritePaths
  defaultTimeRange: '3d' | '7d' | '30d' | 'all'
  notificationSound: boolean
  notificationDesktop: boolean
  theme: 'light' | 'dark' | 'system'
}
```

- [ ] **Step 5: 创建类型入口文件**

创建 `src/types/index.ts`：

```typescript
export * from './session'
export * from './conversation'
export * from './settings'
```

- [ ] **Step 6: Commit**

```bash
git add .
git commit -m "feat: 创建 TypeScript 类型定义"
```

---

## Task 1.6: 创建基础布局组件

**Files:**
- Create: `src/components/layout/AppLayout.tsx`
- Create: `src/components/layout/TabHeader.tsx`
- Create: `src/components/layout/SplitPane.tsx`
- Create: `src/components/layout/index.ts`
- Modify: `src/App.tsx`

- [ ] **Step 1: 创建 layout 目录**

```bash
mkdir -p src/components/layout
```

- [ ] **Step 2: 创建 TabHeader 组件**

创建 `src/components/layout/TabHeader.tsx`：

```typescript
import { cn } from "@/lib/utils"

interface Tab {
  id: string
  label: string
  count?: number
}

interface TabHeaderProps {
  tabs: Tab[]
  activeTab: string
  onTabChange: (tabId: string) => void
}

export function TabHeader({ tabs, activeTab, onTabChange }: TabHeaderProps) {
  return (
    <div className="flex items-center gap-1 border-b bg-background px-4 py-2">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          onClick={() => onTabChange(tab.id)}
          className={cn(
            "px-4 py-2 text-sm font-medium rounded-md transition-colors",
            "hover:bg-muted",
            activeTab === tab.id
              ? "bg-primary text-primary-foreground"
              : "text-muted-foreground"
          )}
        >
          {tab.label}
          {tab.count !== undefined && (
            <span className="ml-2 text-xs opacity-70">({tab.count})</span>
          )}
        </button>
      ))}
    </div>
  )
}
```

- [ ] **Step 3: 创建 SplitPane 组件**

创建 `src/components/layout/SplitPane.tsx`：

```typescript
import { cn } from "@/lib/utils"

interface SplitPaneProps {
  left: React.ReactNode
  right: React.ReactNode
  leftWidth?: number | string
  className?: string
}

export function SplitPane({ left, right, leftWidth = 280, className }: SplitPaneProps) {
  return (
    <div className={cn("flex h-full overflow-hidden", className)}>
      <div
        style={{ width: leftWidth }}
        className="flex-shrink-0 border-r overflow-hidden"
      >
        {left}
      </div>
      <div className="flex-1 overflow-hidden">
        {right}
      </div>
    </div>
  )
}
```

- [ ] **Step 4: 创建 AppLayout 组件**

创建 `src/components/layout/AppLayout.tsx`：

```typescript
import { useState } from "react"
import { TabHeader } from "./TabHeader"

interface AppLayoutProps {
  children: React.ReactNode
}

const TABS = [
  { id: "running", label: "运行中" },
  { id: "management", label: "Session 管理" },
]

export function AppLayout({ children }: AppLayoutProps) {
  const [activeTab, setActiveTab] = useState("running")

  return (
    <div className="flex flex-col h-screen bg-background">
      <header className="flex items-center justify-between px-4 py-2 border-b">
        <h1 className="text-lg font-semibold">Claude Fleet</h1>
        <div className="flex items-center gap-2">
          {/* 后续添加设置按钮 */}
        </div>
      </header>
      <TabHeader tabs={TABS} activeTab={activeTab} onTabChange={setActiveTab} />
      <main className="flex-1 overflow-hidden">
        {children}
      </main>
    </div>
  )
}
```

- [ ] **Step 5: 创建 layout 入口文件**

创建 `src/components/layout/index.ts`：

```typescript
export { AppLayout } from './AppLayout'
export { TabHeader } from './TabHeader'
export { SplitPane } from './SplitPane'
```

- [ ] **Step 6: 更新 App.tsx 使用布局**

编辑 `src/App.tsx`：

```typescript
import { AppLayout } from "@/components/layout"

function App() {
  return (
    <AppLayout>
      <div className="flex items-center justify-center h-full text-muted-foreground">
        Session 内容区域（待实现）
      </div>
    </AppLayout>
  )
}

export default App
```

- [ ] **Step 7: 验证布局**

```bash
npm run tauri dev
```

Expected: 应用显示顶部标题、Tab 导航、占位内容区域

- [ ] **Step 8: Commit**

```bash
git add .
git commit -m "feat: 创建基础布局组件（AppLayout、TabHeader、SplitPane）"
```

---

## Task 1.7: 配置路径别名

**Files:**
- Modify: `tsconfig.json`
- Modify: `vite.config.ts`

- [ ] **Step 1: 配置 TypeScript 路径别名**

编辑 `tsconfig.json`，添加 paths 配置：

```json
{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"]
    }
  }
}
```

- [ ] **Step 2: 配置 Vite 路径别名**

编辑 `vite.config.ts`：

```typescript
import { defineConfig } from "vite"
import react from "@vitejs/plugin-react"
import path from "path"

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
})
```

- [ ] **Step 3: 安装 @types/node（如果需要）**

```bash
npm install -D @types/node
```

- [ ] **Step 4: 验证路径别名**

确保所有 `@/` 导入正常工作：

```bash
npm run tauri dev
```

Expected: 应用正常运行，无路径错误

- [ ] **Step 5: Commit**

```bash
git add .
git commit -m "feat: 配置 TypeScript 和 Vite 路径别名"
```

---

## Phase 1 完成检查

- [ ] **验证所有功能**

```bash
npm run tauri dev
```

检查：
- 应用窗口正常打开
- Tab 切换正常
- Tailwind 样式正常
- shadcn/ui 组件可用
- TypeScript 编译无错误

- [ ] **Final Commit**

```bash
git add .
git commit -m "complete: Phase 1 项目初始化完成"
```
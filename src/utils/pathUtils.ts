/**
 * 构建目录树结构
 */
export interface TreeNode {
  path: string
  name: string
  children: TreeNode[]
  sessionCount: number
  isLeaf: boolean
}

/**
 * 从路径列表构建树结构
 */
export function buildPathTree(paths: string[]): TreeNode {
  const root: TreeNode = {
    path: '',
    name: 'root',
    children: [],
    sessionCount: 0,
    isLeaf: false,
  }

  for (const path of paths) {
    const parts = path.split(/[/\\]/).filter(Boolean)
    let current = root

    for (const part of parts) {
      let child = current.children.find((c) => c.name === part)
      if (!child) {
        child = {
          path: current.path ? `${current.path}/${part}` : part,
          name: part,
          children: [],
          sessionCount: 0,
          isLeaf: false,
        }
        current.children.push(child)
      }
      current = child
    }
  }

  return root
}

/**
 * 获取路径的最后一部分
 */
export function getLastPathSegment(path: string): string {
  const parts = path.split(/[/\\]/).filter(Boolean)
  return parts[parts.length - 1] || path
}
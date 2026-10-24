# UI_SPEC

视觉基准：浅灰左栏 + 白色阅读区的 macOS 双栏布局，原生交通灯，克制边框与阴影。
颜色/圆角/字号集中在 `src/styles/tokens.css`（light / dark + 5 个强调色）。

## 布局

```
MainWindow
├── Sidebar (默认 300px, 220–420px)
│   ├── Header        「Notes」+ 新建按钮
│   ├── SearchInput   关键词；⌘F 聚焦；150ms 防抖
│   ├── FilterChips   全部 | 收藏
│   ├── NoteList      独立滚动；标题/摘要/相对时间/标签；空态与「加载更多」
│   └── TagList       固定下区，独立滚动；全部标签 + 计数；行内 TagEditor
├── SidebarSplitter   拖拽调宽，受窗口宽度约束
└── ContentPane
    ├── NoteHeader    标题（阅读为 h1 / 编辑为输入框）、收藏、标签、
    │                 复制全文、编辑/完成、删除
    ├── NotePreview   阅读：分段渲染 Markdown，代码块各自带「复制代码」
    ├── NoteEditor    编辑：CodeMirror 6 + markdown()
    └── Footer        更新时间 + 保存状态（保存中/已保存/未保存·将重试）
```

## 组件映射

| 计划中的组件 | 现状 |
| --- | --- |
| `Sidebar` / `SidebarSplitter` / `ContentPane` | 已实现 |
| `NoteList` / `NoteHeader` / `NoteEditor` / `NotePreview` / `CodeBlock` | 已实现 |
| `SearchInput` | 已实现 |
| `SearchResults` | 合并进 `NoteList`（结果列表即笔记列表，无独立视图） |
| `TagList` / `TagEditor` | 已实现（行内新建标签） |
| `SettingsDialog` / `AppearanceSettings` / `LibrarySettings` | 已实现 |

## 交互约定

- 单击笔记在右侧打开；默认阅读模式，`编辑`/`⌘E` 进入源码编辑，`Esc` 退出并 flush。
- 代码块复制只写代码原文（去掉围栏与语言标记）。
- 预览禁用原始 HTML；外链通过 opener 插件打开。
- 主题、强调色、字号、左栏宽度存 Tauri Store（浏览器开发时退回 localStorage），
  不写入笔记数据库。
- 空态：无选中（引导新建）、搜索无结果（清除筛选 + 新建）、数据库错误（顶部横幅）。

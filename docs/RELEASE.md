# 发行与安装（v0.1.0）

本文件面向拿到构建产物的使用者与打包的人。产品验收标准仍以
`openspec/changes/v1-notes-manager/` 为准。

## 构建产物

```bash
pnpm install
pnpm tauri build                              # 本机架构：.app + .dmg
pnpm tauri build --target universal-apple-darwin   # Intel + Apple Silicon 通用包
```

产物位置：

| 目标 | 路径 |
| --- | --- |
| 本机架构 `.app` | `src-tauri/target/release/bundle/macos/Notes.app` |
| 本机架构 `.dmg` | `src-tauri/target/release/bundle/dmg/Notes_0.1.0_x64.dmg` |
| 通用 `.app` | `src-tauri/target/universal-apple-darwin/release/bundle/macos/Notes.app` |
| 通用 `.dmg` | `src-tauri/target/universal-apple-darwin/release/bundle/dmg/Notes_0.1.0_universal.dmg` |

通用包需要先 `rustup target add aarch64-apple-darwin`。当前构建**只有 ad-hoc 签名**
（`codesign --force --deep --sign -`），没有 Developer ID 与 notarization——这是 v1 的
范围决定（design D20：只做本机自用），不是待办：本机可以直接运行，拷到别的机器会被
Gatekeeper 拦下，且这不是受支持的分发方式。首次手动放行：

```bash
xattr -dr com.apple.quarantine /Applications/Notes.app   # 或右键 → 打开
```

## 安装与首次启动

1. 打开 `.dmg`，把 `Notes.app` 拖到「应用程序」。
2. 首次启动**不会弹主窗口**——这是刻意的：应用常驻菜单栏，用菜单栏图标或全局
   快捷键唤出。首次运行会弹一次原生提示，说明快捷键与菜单栏位置（只弹一次，
   标记文件在 `~/Library/Application Support/com.scopeva.notes/onboarding-shortcut-tip`）。
3. 首次启动在 `~/Library/Application Support/com.scopeva.notes/notes.db`
   创建笔记库并完成迁移。

## 快捷键与退出

| 操作 | 默认 |
| --- | --- |
| 显示 / 隐藏主窗口 | `⇧⌘D`（设置里可改，冲突时回退默认并提示） |
| 新建笔记 | `⌘N` |
| 搜索 | `⌘F` |
| 打开设置 | `⌘,` |
| 退出 | 菜单栏「退出」或 `⌘Q` |

关闭窗口是隐藏而不是退出；`⌘Q` 与菜单栏退出都会先让前端保存，再弹确认框，
保存失败则不退出。

## 笔记库位置与备份

- 默认位置：`~/Library/Application Support/com.scopeva.notes/notes.db`。
- 指针配置：同目录的 `config.json`（`libraryPath`）。自定义路径不会被复制，
  切换后直接读写新文件。
- 设置 →「笔记库」可查看当前绝对路径、笔记数量与文件大小，并可打开、新建、
  迁移、备份。备份用 `VACUUM INTO`，**拒绝覆盖已存在的文件**。
- 配置指向的库失效时，启动会回退默认库并在设置页显示横幅，提供「重新选择…」
  与「使用默认库」。
- 库在 Dropbox / OneDrive / Google Drive / iCloud Drive / Nextcloud 等同步目录时
  会有提示：同一时间只在一台设备写入。多设备同时写入会产生 SQLite 锁冲突，
  表现为可读的「数据库被占用」错误，而非静默覆盖。

## 已知限制

- **只在本机使用**：ad-hoc 签名，不做 Developer ID 签名与公证（9.2 移出 v1 范围，
  design D20）。产物可以拷到别的机器手动放行运行，但那不是受支持的分发方式。
- Intel 包在本机实测启动；arm64 slice 只做了产物校验，未在 Apple Silicon 上运行。
- 驻留测量：已有 10 分钟空闲采样（0.00% CPU、RSS 52.3 → 41.7 MB 后稳定）与启动里程碑
  （进程 0.19 s、开库 0.74 s），未跑满 24 小时；需要更长曲线时执行
  `scripts/perf-check.sh soak 24`。
- macOS 实机交互（快捷键连按、多显示器、Spaces/Stage Manager、菜单栏观感）与端到端
  UI 冒烟由用户接受、**尚未人工验证**（6.9/8.3，design D20）；使用中发现问题先从
  `openspec/changes/v1-notes-manager/verification.md` 的对应场景查起。
- 预览高亮超过 2 万字符的代码块会退回纯文本；编辑器内代码围栏不着色（由预览承担）。
- 只有「笔记」一个实体，没有独立代码片段库、没有多窗口、没有快速面板。

## 卸载

```bash
rm -rf /Applications/Notes.app
# 数据（删除前请先确认已备份）
rm -rf ~/Library/Application\ Support/com.scopeva.notes
~/Library/LaunchAgents/com.scopeva.notes.plist   # 若开启过「开机启动」，先在设置里关闭
```

## 开发期自检

```bash
pnpm vue-tsc --noEmit && pnpm vitest run && pnpm vite build
cd src-tauri && cargo test --test database --test library --test failures --test recovery -- --test-threads=1
scripts/perf-check.sh startup            # 启动里程碑（进程 / 库打开 / WebKit）
scripts/perf-check.sh soak 24            # 24h 驻留采样，Ctrl-C 结束
```

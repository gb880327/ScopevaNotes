# 发行与安装（v0.2.0）

本文件面向拿到构建产物的使用者与打包的人。产品验收标准仍以
`openspec/specs/`（基线）与 `openspec/changes/archive/2026-10-24-v1-notes-manager/`（决策与验收记录）为准。

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

## 从其他应用迁移笔记（批量导入）

菜单栏菜单里的「导入 Markdown…」，或设置面板最下方的同一入口。

1. 选择目录（只读：原目录里的文件不会被修改、移动或删除）。
2. 预览统计：待导入 / 已导入跳过 / 跳过的隐藏条目 / 引用了本地文件 / 无法读取。
3. 导入：200 条一批提交，进度条实时更新，可随时取消（停在批边界，已提交的批次保留）。
4. 报告：成功、跳过、逐文件失败原因（含路径）、按 GB18030 解码数、未迁移的本地文件引用数、
   耗时与本批标签。可直接「删除本批」整批撤销——走回收站，仍可恢复。

行为要点与限制：

- 只导入 `.md` / `.markdown`；隐藏目录（`.obsidian`、`.trash` 等）、`node_modules`、
  符号链接会被跳过并计数。
- **不迁移附件**：`![[img/a.png]]`、`![](...)` 等引用在正文里逐字保留，但图片文件本身
  不会被复制。报告会给出「引用了本地文件（不迁移）」的笔记数。**请自行保留原目录**，
  否则这些链接会失效（本版最大的已知限制）。
- 不解析 Obsidian 扩展语法（`[[链接]]`、`==高亮==`、`%%注释%%`、callout、块引用）：
  原文完整保留，因此预览里仍会看到这些裸语法。这是刻意选择——导入时改写正文不可逆。
- frontmatter 只认 `title` / `tags` / `created`，其余键丢弃；时间顺序沿用文件的
  修改时间，不按导入时间重排。
- 不做同步、目录监听或增量导入。重跑同一目录是安全的：已导入的文件按来源路径跳过。
- 同一天两次导入共用 `import-YYYY-MM-DD` 这一撤销粒度：撤销会一起撤销当天的两批。
- 大库（1,000 / 5,000 条）实测耗时见 `CHANGELOG.md` 0.2.0；导入期间另一次自动保存
  的等待上界是单批间隔（5,000 条实测 131 ms），不会撞上 5 秒的 `busy_timeout`。

## 回收站与永久删除

「回收站」里的笔记不进普通列表、搜索与标签计数，每条显示删除时间。可以恢复，
也可以永久删除——单条或一次清空。两者都会先在原地确认并写明「不可恢复」，
确认后无法撤回（应用里不留第二份副本）。

永久删除会一并移除正文、标签关联与搜索索引，库里不再有这条记录；它也会释放导入时
记下的来源文件，之后重新导入那个文件会得到一条新笔记（还留在回收站里的笔记不算，
它仍然占着来源，重导会被跳过）。

**永久删除不会立刻缩小库文件。** SQLite 只是把腾出来的页放进 freelist 供后续写入复用，
文件大小保持不变；应用不会为回收磁盘而执行 `VACUUM`（那会重写整个库文件，期间风险
大于收益）。需要真实回收空间时，自行在应用关闭后对库文件执行 `VACUUM`。

## 标签维护

标签就在左栏列表里维护，不另开窗口：鼠标移到某一行，行尾出现「重命名 / 删除」，
点行首色点换颜色。点名称仍然是按该标签过滤。

删除标签会在行内先问一次，文案带「将影响 N 条笔记」——这个数字只算未删除的笔记。
确认后只摘掉这些笔记身上的该标签，笔记本身（含正文、时间戳、收藏状态）不受影响。
「全部标签」右侧的数字是全部标签的可见笔记数之和，同一条笔记若有多个标签会被重复计入。

点标签名按它过滤笔记，**一次只能选一个**：选另一个标签会替换当前的，再点一次同一个
标签就把过滤清掉。标签过滤可以和搜索关键词、收藏筛选叠加（同时成立才出现）。

## 速记（菜单栏左键）

点菜单栏图标**左键**呼出速记面板；主窗口改由右键菜单「显示主窗口」、`⇧⌘D`、
Dock 图标或二次启动唤出。面板是一个无标题栏的小窗口（480×320，最小 360×220），
置顶并跟随所有桌面/Space，点面板外任何地方就自动收起。

一条速记就是一个彩色圆点加一段纯文本。**最多 12 个**——到上限时 `+` 变灰并提示
「速记最多 12 个」，删掉一条才能再加。圆点有 7 种颜色，点「更多」改标题、换颜色、
复制或删除；复制得到的是独立副本（标题追加「副本」），改一个不影响另一个。

正文会自动保存，底部会告诉你「正在保存…」「已保存」或「保存失败：…」。写库失败时
文字不会丢：草稿留在本机，修好问题（例如库被别的程序占着）后继续编辑即可重试；
即使应用崩溃或断电，下次打开会发现草稿比库里的新，自动恢复。切 Tab、按 Esc、
点窗口外、关面板、退出应用都会先把当前输入存下来再走。

**速记是只进不出的沙盒**：不能转成笔记、不能导出、没有回收站，删除永久不可恢复
（删除前会问一次，文案写明「不可恢复」）。需要长期保存的内容请在主窗口里写成笔记。

换库（新建 / 打开 / 迁移 / 回退到默认库）后，面板列出的是新库的速记；上一个库里
没保存完的草稿按孤儿丢弃，不会写进新库。

## 快捷键与退出

| 操作 | 默认 |
| --- | --- |
| 显示 / 隐藏主窗口 | `⇧⌘D`（设置里可改，冲突时回退默认并提示） |
| 速记面板 | 菜单栏图标左键（无快捷键，Esc 收起） |
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
  `openspec/changes/archive/2026-10-24-v1-notes-manager/verification.md` 的对应场景查起。
- 预览高亮超过 2 万字符的代码块会退回纯文本；编辑器内代码围栏不着色（由预览承担）。
- 速记只有纯文本（没有 Markdown 预览、没有排序或置顶、没有导出/转笔记的出口），
  上限 12 硬编码在 Rust 侧，删除不可撤销。
- 只有「笔记」一个实体，没有独立代码片段库、没有多窗口。速记是唯一的第二个窗口，
  它只承载便签文本，不是笔记界面。
- 批量导入不迁移附件、不解析 Obsidian 扩展语法、不做同步/目录监听；
  同日批次共用撤销标签（见上一节与 `openspec/changes/archive/2026-10-24-add-markdown-import/design.md`）。

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

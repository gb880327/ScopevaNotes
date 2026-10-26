## MODIFIED Requirements

### Requirement: 笔记数据模型

系统 MUST 只提供一种笔记内容实体 `Note`，包含稳定 id、标题、Markdown 正文、收藏标记、创建时间、更新时间与零到多个标签；MUST NOT 引入独立的代码片段实体或第三栏视图。id MUST 为 UUIDv7，时间戳 MUST 为 Unix 毫秒（UTC）。

唯一的限定例外是**速记（Quick Tab）**：它是独立于 `Note` 的第二实体，MUST 与笔记体系完全隔离——MUST NOT 存储于 `notes`、`notes_fts` 或 `note_tags`，MUST NOT 支持标签，MUST NOT 出现在笔记列表、搜索结果、标签计数、回收站或导入批次中，MUST NOT 提供任何自动或半自动进入笔记体系的通道（含「转换为笔记」）。速记的语义由 `quick-tabs` capability 定义，本条只约束边界：`Note` 的字段、软删除与恢复、标签、回收站语义不因速记存在而改变。

#### Scenario: 创建笔记只产生一个实体
- **WHEN** 用户新建笔记并输入标题与正文
- **THEN** 数据库只新增一条 `notes` 记录与其关联的 `note_tags` 记录，不产生片段或附件记录

#### Scenario: 空正文合法
- **WHEN** 用户新建笔记后立即保存且未输入任何正文字符
- **THEN** 保存成功，笔记以空正文存在于列表中，标题为空时以「无标题」呈现

#### Scenario: 速记不扩张笔记实体
- **WHEN** 用户创建并编辑若干速记后打开笔记列表与搜索
- **THEN** 列表与搜索结果只包含 `notes` 中的内容，速记既不出现在列表也不影响标签计数

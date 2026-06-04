# fsrs-cli 用户指南

> 命令行里的第二大脑：用 FSRS 算法帮你记住所有想记住的东西。

---

## 这是什么？

`fsrs-cli` 把你的 Markdown 笔记变成"知识卡片"，用 [FSRS](https://github.com/open-spaced-repetition/fsrs4anki) 间隔重复算法告诉你什么时候该复习什么。

所有数据存在本地 `.fsrs/` 目录里，不联网也能用。AI 功能是可选的。

---

## 安装

需要 Rust 工具链 1.75+。

```bash
# 仅核心功能
cargo install --path . --release

# 包含 AI 功能
cargo install --path . --release --features ai
```

构建后二进制在 `target/release/fsrs-cli`，建议加入 `$PATH`。

---

## 快速开始

### 1. 初始化记忆库

在你存放笔记的目录下执行：

```bash
fsrs-cli init
```

你会看到：

```
✓ 记忆库已初始化于 .fsrs
  数据库:   .fsrs/memory.db
  索引:     .fsrs/tantivy
  元数据:   .fsrs/meta.json
```

这会创建 `.fsrs/` 目录，里面包含 SQLite 数据库和搜索索引。

### 2. 写卡片

在你的笔记 Markdown 文件里，用以下格式写卡片：

```markdown
---
tags: [rust, ownership]
---

Rust 的所有权系统是其内存安全的核心。

<!-- card -->
Q:: Rust 中一条值同时只能有几个可变引用？
A:: 恰好一个。这是借用检查器在编译期强制执行的规则。
<!-- card -->

<!-- card -->
Q:: 什么是借用？
A:: 用 & 表示不可变借用，&mut 表示可变借用，借用不获取所有权。
<!-- card -->
```

- 文件顶部的 `---` 包裹的是 YAML frontmatter，里面的 `tags` 会自动应用到所有卡片
- `<!-- card -->` 是开始标记
- `Q::` 后面是问题
- `A::` 后面是答案
- 再次 `<!-- card -->` 结束

中间没有 `Q::`/`A::` 的普通笔记会被忽略。

### 3. 导入

```bash
# 导入单个文件
fsrs-cli import notes/rust-ownership.md

# 导入整个目录（包括子目录）
fsrs-cli import notes/
```

```
📁 找到 2 个 Markdown 文件
  ✓ notes/advanced/lifetimes.md - 新增 2, 更新 0

✓ 导入完成
  新增: 2 张
  更新: 0 张
  跳过: 3 张
  总计: 2 张
```

- **新增**: 文件中新增的卡片
- **更新**: 已存在但问题被改了
- **跳过**: 已存在且问题未变（自动去重）

### 4. 复习

```bash
fsrs-cli review
```

每张卡片会按以下流程出现：

1. 显示问题
2. 你在心里回答
3. 按 **Enter** 显示答案
4. 自我评分：1（忘了） / 2（困难） / 3（一般） / 4（简单）
5. 算法自动安排下次复习时间

随时按 `q` 退出，已评分的进度会保存。

---

## 命令参考

### `init` — 初始化记忆库

```bash
fsrs-cli init
```

创建 `.fsrs/` 目录、SQLite 数据库、Tantivy 搜索索引和 `meta.json`。

**已初始化会提示**：

```
⚠ 记忆库已存在于 .fsrs
```

不会覆盖现有数据。

---

### `import` — 导入卡片

```bash
fsrs-cli import <路径>           # 单个文件或目录
```

**路径支持**：
- `.md` 文件：解析其中所有卡片
- 目录：递归扫描所有 `.md` 文件，非 `.md` 静默忽略
- 不存在的路径：报错退出

**导入失败会被跳过**：
- 问题或答案为空 → 跳过
- frontmatter 不是合法 YAML → 仍尝试解析，回退为空 tags

**重复导入是安全的**：
- 问题文本未变 → 跳过
- 问题变了 → 更新内容，重置复习进度

---

### `csv-import` — 从 CSV 导入

```bash
fsrs-cli csv-import <file.csv>
```

支持 tab 分隔的 CSV 文件（**无表头**）：

| 形式 | 列 | 说明 |
|---|---|---|
| 2 列 | `问题\t答案` | 标签为空 |
| 3 列 | `问题\t答案\t标签` | 多个标签用 `\|` 分隔 |

**特殊字符：**
- 答案中的字面 `\n` 会被转换为真换行（多行答案）
- 标签缺省或空 → `tags = []`

**示例 `cards.csv`：**
```
Rust 中 mutable 借用规则？	同一作用域内\n只能有一个 &mut	Rust|语法
什么是 FSRS 算法？	Free Spaced Repetition Scheduler
Go 的 defer 执行顺序？	LIFO 顺序	Go|语法
```

执行：
```bash
fsrs-cli csv-import cards.csv
```

**与 `import` 行为一致：** 重复导入按问题哈希去重，相同的跳过、变化的更新并重置复习进度。来源在卡片元数据中记录为 `cards.csv`。

---

### `make` — 交互式制作卡组

```bash
fsrs-cli make <output.md>        # 必填输出文件
```

内部 TUI editor（参考 glow 最简风格），逐张输入卡片：

```
📝 制作卡组 → notes/anki-export.md

问题: Rust 中 mutable 借用规则？
答案 (逐行输入 Markdown；单独输入 :end 结束；Ctrl+D 也可结束)
  > 同一作用域内
  > 只能有一个 &mut
  > :end
标签 (| 分隔，可留空): Rust|语法
✓ 已添加第 1 张卡片

继续添加下一张? [Y/n]: y
...
```

**字段说明：**

| 字段 | 必填 | 规则 |
|---|---|---|
| 问题 | ✅ | trim 后非空 |
| 答案 | ✅ | 多行，输入 `:end` 单独一行结束；也支持 `Ctrl+D` |
| 标签 | ❌ | `\|` 分隔，空 → 无标签 |

**答案支持 Markdown**（粗体/斜体/标题/列表/代码块），存储为原始 Markdown。

**写入行为：**
- 文件不存在 → 创建并写入
- 文件存在 → 在末尾追加（前面加空行分隔）
- 写入完成后**自动调用 import** 入库
- 中途 `Ctrl+C` 退出 → 全部丢弃，文件不变

**示例：**
```bash
fsrs-cli make notes/daily.md
fsrs-cli make deck.md
```

---

### `review` — 复习到期卡片

```bash
fsrs-cli review                  # 复习所有到期卡片
fsrs-cli review --limit 10       # 最多复习 10 张
```

每张卡片显示时附带当前可检索性（保留率）。评分键：

| 键 | 评分 | 含义 | 何时选 |
|---|------|------|--------|
| `1` | Again | 完全忘了 | 看到答案才想起来 |
| `2` | Hard | 困难 | 想起来了但很费劲 |
| `3` | Good | 一般 | 略加思索能答出 |
| `4` | Easy | 简单 | 立刻答出且很确定 |

其他键：

- `q` 退出（已评分的会保存）
- `Enter` 显示答案前不计入退出

---

### `search` — 全文搜索

```bash
fsrs-cli search "所有权"             # 中文搜索（jieba 分词）
fsrs-cli search "lifetime"          # 英文搜索
fsrs-cli search "rust" --limit 5    # 限制结果数
```

```
🔍 找到 2 张匹配的卡片

  Q: [3] 所有权转移（move）发生在什么时候？ (DUE) 可检索性: 0%
    来源: notes/rust-ownership.md
    标签: rust, ownership
```

输出字段：

- **`(DUE)`** — 红色：当前可复习
- **`X 天后到期`** — 距离下次复习的天数
- **`可检索性: X%`** — FSRS 估计你现在能想起这张卡片的概率
- **`来源`** — 原始 Markdown 文件相对路径
- **`标签`** — frontmatter 标签

**搜索引擎**：默认用 Tantivy（快、支持中文分词）。如果索引文件损坏，自动回退到 SQLite LIKE 搜索（慢、但能用）。

---

### `edit` — 编辑单张卡片

```bash
fsrs-cli edit "所有权"            # 按问题关键词搜索
fsrs-cli edit "aec90ae2"          # 按 ID 前缀
```

打开 `$VISUAL` 或 `$EDITOR`（默认 vim）编辑临时文件。格式：

```
# 问题
...

# 答案
...

# 标签
tag1, tag2
```

保存退出后写回数据库。**重要**：如果问题文本被改动，复习进度会被重置（防止用旧预测安排复习）。

---

### `update` — 重新导入已存在的文件

```bash
fsrs-cli update notes/rust-ownership.md              # 默认：问题变了重置调度
fsrs-cli update notes/rust-ownership.md --keep-schedule   # 保留调度
```

`update` 与 `import` 的区别：

| 行为 | import | update |
|------|--------|--------|
| 新增卡片 | ✅ | ✅ |
| 问题未变 | 跳过 | 跳过 |
| 问题变了 | 更新+重置调度 | 更新+重置调度（默认） / 保留调度（`--keep-schedule`） |
| 删除的卡片 | 保留 | 保留 |

`--keep-schedule` 适用于：手抖改了几个字但想保留复习历史。

---

### `stats` — 学习统计

```bash
fsrs-cli stats                    # 基础统计
fsrs-cli stats --period 30        # 最近 30 天详细统计
```

**基础输出**：
```
📊 学习统计
  总卡片数:     5
  待复习:       5
  总复习次数:   0
```

**带 `--period` 额外显示**：
```
📈 最近 7 天
  新增卡片:     5
  复习次数:     0
  平均可检索性: 100.0%
```

---

### `optimize` — 查看 FSRS 状态

```bash
fsrs-cli optimize
```

显示当前复习数据是否足够、给出简单的优化建议。**注意**：当前实现用的是默认 FSRS 参数，工具本身不重新训练参数。

复习记录少于 10 条时会提示"数据不足"。

---

### `review-drafts` — 审核 AI 生成的草案

```bash
fsrs-cli review-drafts
```

对每张 AI 生成的草案：

```
────────────────────────────────────────
  Q: [abc12345]
  什么是 Rust 的生命周期？

  A: ...
    来源: notes/rust-ownership.md
  操作: [y] 通过  [n] 拒绝  [q] 退出  [Enter] 跳过
```

- `y` 通过：转成正式卡片，初始化 FSRS 状态
- `n` 拒绝：标记为已拒绝
- `q` 退出
- `Enter` 跳过（下次还能看到）

---

### `ai` — AI 子命令

**需要带 `--features ai` 编译的版本**。所有 AI 命令都调用 OpenAI API（或兼容服务），需要设置 `OPENAI_API_KEY` 环境变量。

#### `ai generate` — 生成卡片

```bash
fsrs-cli ai generate article.md              # 从文件
fsrs-cli ai generate "Rust 借用规则"         # 从主题
fsrs-cli ai generate paper.pdf --pdf         # 从 PDF（需要 --features pdf）
```

AI 返回的卡片先存为"草案"，需用 `review-drafts` 审核。

#### `ai tag` — 自动打标签

```bash
fsrs-cli ai tag <card-id>          # 单张
fsrs-cli ai tag --batch            # 批量处理所有未标记卡片
```

#### `ai expand` — 扩充答案

```bash
fsrs-cli ai expand <card-id>
```

调用 AI 给出更详细的答案版本，写回卡片。

#### `ai health` — 健康检查

```bash
fsrs-cli ai health
```

扫描并报告：
- 答案过短（< 20 字符）
- 问题高度相似（重复卡片）
- 超过 30 天未复习

#### `ai review-suggest` — 热度推荐

```bash
fsrs-cli ai review-suggest
```

按 `访问次数 × (1 - 可检索性)` 排序，列出最该复习的卡片。

---

## 卡片格式详解

### Frontmatter

```markdown
---
tags: [rust, ownership, lifetime]
---
```

`tags` 是数组，会应用到该文件所有卡片。

### 卡片块

```markdown
<!-- card -->
Q:: 问题文本
   跨行继续
A:: 答案文本
   跨行继续
<!-- card -->
```

- 块内空行会被保留为换行
- 没有 `Q::` 或 `A::` 的块会被跳过
- 文件末尾未关闭的块也会被尝试解析

### 完整示例

```markdown
---
tags: [rust]
---

# 所有权

<!-- card -->
Q:: 一条值能同时被多少个不可变引用借用？
A:: 任意多个，只要没有可变借用同时存在。
<!-- card -->

<!-- card -->
Q:: 一条值能同时被多少个可变引用借用？
A:: 恰好一个。
<!-- card -->
```

---

## 存储结构

```
your-project/
├── .fsrs/
│   ├── memory.db       # SQLite 数据库
│   ├── tantivy/        # 搜索索引
│   ├── meta.json       # 元数据 + 默认配置
│   └── config.toml     # （可选）用户配置
├── notes/
│   └── *.md            # 你的源文件，原位不动
```

### meta.json

```json
{
  "version": "0.1.0",
  "schema_version": 1,
  "created_at": "2026-06-03T23:31:23Z",
  "config": {
    "model": "gpt-4o-mini",
    "desired_retention": 0.9,
    "daily_limit": 50,
    "temperature": 0.3
  }
}
```

不要手动修改 schema_version。

---

## 复习评分指南

FSRS 算法对评分非常敏感。**一致性比"给自己面子"更重要**。

### 评分原则

**1 (Again)**：看到答案前完全没想起来，或想错了。
**2 (Hard)**：想起来了但犹豫、不确定。
**3 (Good)**：能答出，过程中有一两次停顿或细节不确定。
**4 (Easy)**：立刻、流畅、完全正确。

### 常见错误

❌ **"我大概能想起来" → 选 3**  
✅ 应该是 2。想起来 ≠ 确定。

❌ **"我刚才都答对了" → 选 4**  
✅ "答对"是 3 的标准。4 应该是"秒答"。

❌ **看到答案发现答错 → 选 1**  
✅ 立刻按 1，不要补救。算法会根据遗忘频率自动缩短间隔。

### 间隔大致规律

| 评分 | 第一次复习 | 第二次 |
|------|------------|--------|
| 1 (Again) | < 1 天 | 几分钟（学习期） |
| 2 (Hard) | 1-2 天 | ~1 周 |
| 3 (Good) | 3-5 天 | ~2-3 周 |
| 4 (Easy) | 1 周 | ~1 月+ |

具体间隔由 FSRS 算法根据你的历史调整。

---

## 数据安全

### 源文件不受影响

工具只读取 `.md` 文件，**永远不会修改你的源文件**。改卡片用 `edit` 命令在临时文件中改。

### 删除数据

| 操作 | 后果 |
|------|------|
| 删除 `.fsrs/memory.db` | 丢失所有复习记录，可重新 `import` 恢复卡片 |
| 删除 `.fsrs/` | 全部清空，需重新 `init` |
| 删源文件 | 数据库还在，但 `import` 不会再读到 |

### 复习记录不可篡改

AI 永远不能直接修改你的复习记录。AI 生成的卡片需经 `review-drafts` 审核才会进入正式卡片库。

---

## 故障排查

### "记忆库未初始化，请先运行 fsrs-cli init"

当前目录下没有 `.fsrs/memory.db`。要么切到正确目录，要么用 `-p` 指定路径：

```bash
fsrs-cli -p /path/to/memory base review
```

### "未找到 Markdown 文件"

导入路径下没有 `.md` 文件，或所有文件都被解析器跳过了（可能是 frontmatter 问题）。

### Tantivy 报错

如果搜索时报错，工具会自动回退到 SQLite 简单搜索。`import` 失败时检查 `.fsrs/tantivy/` 是否被破坏，删除后 `init` 重建。

### AI 命令 "调用 OpenAI API 失败"

- `OPENAI_API_KEY` 环境变量未设置
- 网络问题
- API key 无效或余额不足

### 想要完全重置

```bash
rm -rf .fsrs/
fsrs-cli init
fsrs-cli import notes/
```

---

## 最佳实践

1. **每天定时 review** — 把 `fsrs-cli review` 加到每天的固定时间
2. **坚持评分一致** — 不要给"虚高"的分数
3. **小批量写卡片** — 一次 5-10 张，写完就 import
4. **使用 frontmatter tags** — 后面搜索和分类都靠它
5. **AI 生成的卡片必须审核** — AI 经常编造错误信息

---

## 与 Anki 的区别

| 特性 | Anki | fsrs-cli |
|------|------|----------|
| 卡片存储 | 私有 SQLite | 通用 SQLite，可读 |
| 源文件 | 私有 .apkg | 原位 Markdown |
| 编辑器 | GUI | 任何文本编辑器 |
| AI 集成 | 插件 | 原生支持 |
| 全文搜索 | 弱 | Tantivy + jieba |

---

## 键盘快捷键速查

### review

| 键 | 作用 |
|---|------|
| `Enter` | 显示答案 |
| `1` `2` `3` `4` | 评分 |
| `q` | 退出（已评分的会保存） |

### search / 其他

按 `Enter` 继续。

---

## 反馈

发现 bug 或有建议，请到项目仓库提 issue。

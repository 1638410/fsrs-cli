# fsrs-cli 设计文档（v3.0）

## 1. 项目概述

`fsrs-cli` 是一个基于 Markdown、采用 FSRS 间隔重复算法的命令行记忆管理工具。支持从 Markdown 导入卡片、交互式复习、全文搜索、内容更新与参数优化。AI 辅助层允许 AI 参与卡片生成、标签、健康检查与自有记忆管理，但严格隔离于人类复习回路。

## 2. 核心设计原则

| 原则 | 说明 |
|------|------|
| 职责隔离 | FSRS 调度器、搜索、存储、AI 辅助严格分层 |
| 卡片为最小单元 | 文件仅作为人类可读容器 |
| 人类可读优先 | 文件名使用可读标题，内容用标记语法 |
| 事务原子性 | 复习过程单卡原子提交，任意中断无状态丢失 |
| 安全默认 | 内容变更主动重置调度状态 |
| 人类复习回路不可侵犯 | AI 永远不能直接修改人类 FSRS 状态或复习日志 |
| AI 辅助在内容端 | AI 负责卡片生成、标签、关联、健康建议，不替人评分 |

## 3. 数据模型

### 3.1 人类侧核心实体

**Card**

| 字段 | 类型 | 说明 |
|------|------|------|
| id | UUID | 主键 |
| source_file | TEXT | 可读路径 |
| card_index | INTEGER | 文件中序号 |
| question | TEXT | 问题 |
| answer | TEXT | 答案 |
| tags | TEXT (JSON) | 标签数组 |
| question_hash | TEXT | 问题文本 SHA256 |
| created_at | TEXT | 创建时间 |
| updated_at | TEXT | 更新时间 |

**ScheduleState**（1:1 关联 Card）

| 字段 | 类型 | 说明 |
|------|------|------|
| card_id | TEXT | 外键 |
| difficulty | REAL | 难度 |
| stability | REAL | 稳定性 |
| last_review | TEXT | 上次复习时间 |
| due_date | TEXT | 到期日 |
| reps | INTEGER | 复习次数 |
| lapses | INTEGER | 遗忘次数 |
| state | INTEGER | New=0/Learning=1/Review=2/Relearning=3 |

**ReviewLog**（不可变）

| 字段 | 类型 | 说明 |
|------|------|------|
| id | INTEGER | 自增主键 |
| card_id | TEXT | 外键 |
| rating | INTEGER | Again=1/Hard=2/Good=3/Easy=4 |
| elapsed_days | INTEGER | 距上次复习天数 |
| scheduled_days | INTEGER | 计划间隔天数 |
| review_time | INTEGER | 复习耗时（毫秒） |
| reviewed_at | TEXT | 复习时间戳 |

### 3.2 AI 侧实体

**AIAccessLog**

| 字段 | 类型 | 说明 |
|------|------|------|
| id | INTEGER | 自增主键 |
| card_id | TEXT | 外键 |
| access_time | TEXT | 访问时间 |
| reason | TEXT | 搜索/生成建议/健康检查 |
| model_name | TEXT | 模型名称 |

**AICardDraft**

| 字段 | 类型 | 说明 |
|------|------|------|
| id | TEXT | 主键 |
| question | TEXT | 问题 |
| answer | TEXT | 答案 |
| tags | TEXT (JSON) | 标签 |
| source_file | TEXT | 来源文件 |
| confidence | REAL | 置信度 |
| source_prompt | TEXT | 生成提示 |
| created_at | TEXT | 创建时间 |
| state | TEXT | draft/approved/rejected |

**AIMemory**（可选，独立 FSRS 实例）

| 字段 | 类型 | 说明 |
|------|------|------|
| id | TEXT | 主键 |
| content | TEXT | 内容 |
| tags | TEXT (JSON) | 标签 |
| created_at | TEXT | 创建时间 |
| updated_at | TEXT | 更新时间 |

### 3.3 状态生命周期

```
New → (首次复习) → Learning → (连续正确) → Review
Review → (遗忘) → Relearning → (重新正确) → Review
```

## 4. 系统架构

```
┌──────────────── CLI Layer (clap) ────────────────────┐
│ init│import│review│search│edit│update│ai│optimize│stats│
├──────────┬──────────┬────────────┬─────────┬─────────┤
│Scheduler │ Reviewer │ Searcher   │ Parser  │AI Helper│
│ (FSRS)   │ (TUI)    │ (tantivy)  │(pulldown│(LLM API)│
├──────────┴──────────┴────────────┴─────────┴─────────┤
│            Storage Layer (SQLite)                     │
│ cards│schedule│reviews│ai_access│ai_drafts│ai_memory │
├──────────────────────────────────────────────────────┤
│          File I/O (Markdown files)                    │
└──────────────────────────────────────────────────────┘
```

- **Scheduler**：纯函数，仅操作 FSRS 算法，不感知文件或搜索
- **SearchEngine**：Tantivy 全文索引 + MoreLikeThis 相似推荐
- **Parser**：pulldown-cmark 解析 Markdown 提取卡片
- **Storage**：SQLite 事务持久化层
- **AI Helper**：通过子命令触发，调用外部 LLM API，结果写入 drafts 或 cards 表（仅 New 状态），绝不接触 schedule/reviews

## 5. 存储层

### 5.1 目录结构

```
<project-root>/
├── .fsrs/
│   ├── memory.db        # SQLite 数据库
│   ├── tantivy/         # 全文搜索索引
│   └── config.toml      # 配置文件（可选）
└── *.md                 # Markdown 记忆文件（原地存储）
```

文件保持原位，数据库记录 `source_file` 相对路径。

### 5.2 SQLite 表结构

```sql
CREATE TABLE cards (
    id TEXT PRIMARY KEY,
    source_file TEXT NOT NULL,
    card_index INTEGER NOT NULL,
    question TEXT NOT NULL,
    answer TEXT NOT NULL,
    tags TEXT,
    question_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(source_file, card_index)
);

CREATE TABLE schedule (
    card_id TEXT PRIMARY KEY REFERENCES cards(id),
    difficulty REAL,
    stability REAL,
    last_review TEXT,
    due_date TEXT,
    reps INTEGER,
    lapses INTEGER,
    state INTEGER
);

CREATE TABLE reviews (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    card_id TEXT NOT NULL REFERENCES cards(id),
    rating INTEGER NOT NULL,
    elapsed_days INTEGER,
    scheduled_days INTEGER,
    review_time INTEGER,
    reviewed_at TEXT NOT NULL
);

CREATE TABLE ai_access (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    card_id TEXT NOT NULL REFERENCES cards(id),
    access_time TEXT NOT NULL,
    reason TEXT,
    model_name TEXT
);

CREATE TABLE ai_drafts (
    id TEXT PRIMARY KEY,
    question TEXT NOT NULL,
    answer TEXT NOT NULL,
    tags TEXT,
    source_file TEXT,
    confidence REAL,
    source_prompt TEXT,
    created_at TEXT NOT NULL,
    state TEXT DEFAULT 'draft'
);

CREATE TABLE ai_memory (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    tags TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE schema_version (version INTEGER PRIMARY KEY);
```

### 5.3 关键索引

- `schedule(due_date)` — 加速到期查询
- `cards(source_file, card_index)` — UNIQUE 约束保证唯一性
- `reviews(card_id, reviewed_at)` — 复习历史查询

## 6. Markdown 解析规范

卡片块由 `<!-- card -->` 标记包围，块内使用 `Q::` 和 `A::` 定义问答。文件可包含普通笔记，解析器忽略非卡片块内容。全局 frontmatter（YAML）定义文件级 `tags`，卡片继承。

```markdown
---
tags: [rust, memory]
---

这是普通笔记。

<!-- card -->
Q:: Rust 中一条值同时只能有几个可变引用？
A:: 恰好一个。
<!-- card -->

更多笔记内容。

<!-- card -->
Q:: 什么是所有权？
A:: 每个值都有一个所有者，值在离开作用域时被丢弃。
<!-- card -->
```

解析基于 `pulldown-cmark`，遇到 `<!-- card -->` 切换状态，提取内部文本，由 `parse_qa_block` 拆解问答。格式错误降级警告，不影响其他卡片。

## 7. CLI 命令

### 7.1 核心命令

| 命令 | 功能 |
|------|------|
| `init` | 在当前目录初始化记忆库（创建 .fsrs/ 及数据库） |
| `import <path>` | 从 Markdown 文件或目录解析卡片并导入 |
| `review [--limit N]` | 交互式复习到期卡片 |
| `search <query> [--limit N]` | 全文搜索卡片，显示 DUE/临期/正常状态 |
| `edit <target>` | 打开外部编辑器修改文件或卡片 |
| `update <path> [--keep-schedule]` | 重新解析文件，更新卡片；默认重置问题变更的调度 |
| `stats [--period]` | 查看学习统计 |
| `optimize` | 用历史复习数据优化 FSRS 参数 |

### 7.2 AI 命令

| 命令 | 功能 |
|------|------|
| `ai generate <topic/file/url>` | 调用 AI 生成卡片草案，存入 ai_drafts |
| `ai tag <card-id>` | 为卡片自动推荐标签与相关卡片 |
| `ai tag --batch` | 批量处理未标记卡片 |
| `ai health` | 扫描知识库，输出内容质量报告 |
| `ai expand <card-id>` | 为一句话卡片补充详细答案 |
| `ai review-suggest` | 基于 AI 访问频率，列出推荐复习的卡片 |
| `review-drafts` | 预览、编辑、通过或删除 AI 生成的草案 |

## 8. 核心流程

### 8.1 复习流程（事务化 TUI）

```
get_next_due_card()
  → 展示题目
  → 用户按键显示答案
  → 评分选择 (1/2/3/4)
  → Scheduler 计算新状态
  → 单卡事务提交
  → 继续下一张
```

游标模式：每次只取一张到期卡片，评分后立即单卡事务提交。任意时刻 Ctrl+C，当前卡片状态未变更，下次复习仍为第一张到期卡片。

评分映射：1-Again, 2-Hard, 3-Good, 4-Easy。

### 8.2 更新与变更检测

1. 解析新文件，按 `(source_file, card_index)` 匹配旧卡片
2. 仅对 `question` 字段计算 SHA256 哈希
3. 若问题哈希变化：
   - 默认：重置调度状态为 `New`
   - `--keep-schedule`：保留状态，终端输出**红色警告**
4. 若仅答案或格式变化：保留调度，仅更新内容

### 8.3 搜索与调度集成

```
Tantivy 搜索 → card IDs + 相似度分数
       ↓
SQLite 批量查询 → schedule 状态
       ↓
合并排序 → 展示（含 DUE/临期/正常标识）
```

### 8.4 AI 生成卡片流程

```
输入 (Markdown/主题/URL)
  → 切片 (~500 token chunks)
  → LLM 生成 (JSON 模式)
  → 解析验证
  → 写入 ai_drafts (state=draft)
  → 人类 review-drafts 审核
  → 通过后转入正式 cards 表
```

### 8.5 AI 健康检查（只读）

扫描所有卡片，检测：
- 答案过短（<20 chars）
- 问题高度相似（Jaccard 相似度 > 0.8）
- 长时间未复习且无 AI 访问记录

生成报告，建议清理/合并/重写，不自动修改数据库。

### 8.6 AI 热度提醒

展示卡片时查询 `ai_access` 表过去 30 天的访问记录：

```
heat = sum(1 / (days_since_access + 1))
```

若 heat > 阈值，显示 🔥 图标及次数。仅作参考信息，不影响复习排序。

## 9. 关键决策

| 问题 | 决策 | 理由 |
|------|------|------|
| AI 调用等同于人类评分 | 拒绝；AI 访问写入独立 ai_access 表 | 保持 FSRS 统计有效性 |
| AI 直接参与人类复习评分 | 禁止；AI 不能触碰 schedule/reviews 表 | 人类记忆曲线由人类维护 |
| AI 对知识库的贡献 | 内容端：生成、标签、关联、健康检查 | 发挥 LLM 优势，不越界 |
| AI 生成卡片质量 | 先入 draft，需人类 review 后转入正式卡片 | 最小信任，防止垃圾数据 |
| 内容变更后调度状态 | 默认重置；--keep-schedule 保留并警告 | 安全优先 |
| 复习中断一致性 | 游标模式，单卡事务提交 | 任意 Ctrl+C 无丢失 |
| 搜索与调度脱节 | 搜索结果自动关联 due_date | 提升信息密度 |
| 参数优化时机 | 手动命令；复习结束时提示建议 | 避免无意义计算 |
| AI 相似度检测 | Jaccard 相似度（关键词级别） | 轻量，无需嵌入模型 |
| AI 热度指标 | 时间加权衰减公式 | 平衡频率与时效性 |

## 10. 依赖清单

```toml
[dependencies]
# Core
clap = { version = "4", features = ["derive"] }
dialoguer = "0.11"
console = "0.15"
anyhow = "1"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = "0.3"

# FSRS
fsrs = "6.0"

# Storage
rusqlite = { version = "0.31", features = ["bundled"] }

# Search
tantivy = "0.22.1"
tantivy-jieba = "0.19.0"

# Markdown
pulldown-cmark = "0.11"

# NLP
jieba-rs = { version = "0.8", features = ["tfidf"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yml = "0.0.12"

# Utilities
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
sha2 = "0.10"
walkdir = "2"

# AI (optional, feature-gated)
tokio = { version = "1", optional = true }
reqwest = { version = "0.12", optional = true }
async-openai = { version = "15", optional = true, default-features = false, features = ["chat-completion"] }

[features]
default = []
ai = ["tokio", "reqwest", "async-openai"]
```

## 11. 未来扩展

- 存储层接口抽象为 trait，支持远端同步或 CRDT 协作
- Markdown 双向链接 `[[]]` 构建显式知识图谱
- AI draft 自动接纳阈值（高置信度自动发布）
- 基于 AI 访问热度的优先级调整（需人工确认）
- 插件体系：自定义评分界面、导出格式（Anki apkg）
- 多用户协作时 AI 充当冲突协调者

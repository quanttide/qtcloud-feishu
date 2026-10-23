# 增量消费模型

本文件定义一套与具体数据源无关的消费模型：一个 Source 产生的数据，如何被多个独立 Consumer 订阅、消费，并把各自的进度可靠地记下来。它回答 qtcloud-feishu#1，取代实验版本里 `Target` / `Cursor` / `watch` / `poll` 那一套与飞书 API、数据库耦合的写法——迁移实验结论，不迁移实验实现。

## 定位

现有三层各管一段：`resources/` 说的是飞书的话（动作登记即契约），`engine.rs` 把定义翻译成一串活动，`flow.rs` 在活动之上推格。缺的是中间这层：一次拉回来的数据怎么交给多个互不相干的处理者，各自记到哪、失败从哪重来。没有它，每加一个功能就要自己实现一遍游标与增量，`engine.rs` 里那条 `增量` 声明也会长成互不兼容的几种。

模型是一条链：Source → Observation → Subscription → Consumer → Checkpoint。

```text
Source ──摄取，一次 IO──▶ Observation Log ──按订阅分发──▶ Consumer A ──▶ Checkpoint A
                                      ├────────────────▶ Consumer B ──▶ Checkpoint B
                                      └────────────────▶ Consumer C ──▶ Checkpoint C
```

三条不变式，全文的判断都以它们为准。摄取与消费分离，摄取一轮只读一次来源；每个 (Consumer, Subscription) 有一个互不干扰的 Checkpoint；Checkpoint 单调前进，只由处理成功的批次推进。

## 五个概念

### Source

Source 是一个可被拉取的地址，由平台、类型与范围三段决定，逻辑标识写作 `平台:类型:范围`，例如 `feishu:im:oc_xxx`、`feishu:mail:hr@example.com:INBOX`、`github:issue:quanttide/qtcloud-feishu`。落盘时把分隔符换成目录层级，得 `log/feishu/im/oc_xxx/…`。

Source 声明四件事：标识、顺序键（Position 按哪个字段排）、扫描方式（按位置查询，还是只能窗口扫描）、归一化函数（原始负载怎么变成 Observation）。顺序键必须选来源能排序、能按范围查询的那个字段，而不是业务上想要的那个字段——Position 换不回一次查询，增量就退化成全量。Source 不知道谁在消费它，也不持有任何 Checkpoint，这是解耦的唯一硬约束。

### Observation

Observation 是从 Source 取出的一条事实，分两层：证据与负载。证据是 `raw/` 里那页原样响应的路径加外部标识，负载是我们拍平后的最小字段。一条记这些：`source`、`external_id`、`kind`、`created_at`、`updated_at`、`parent`（父消息、所属会话、被回复或引用的对象）、`url`（能回到平台原文的链接）、`raw_ref`，加归一化后的 `payload`。

Observation 与 Event 在词汇上分开，在层级上不分。本模型只定义 Observation，因为拉回来的是记录而不是既成事件：消息可编辑、可撤回，邮件可被移动，issue 可被关掉又重开。Event 预设不可变且已经过解释，那是 Consumer 的产出，落到 State 或下游，不进入订阅关系。第一阶段只有 Observation。

### Subscription

Subscription 是一个 Consumer 对某个 Source 的订阅关系，写清楚四样：source、filter（哪些 observation 算命中）、scope（范围，如只收某人发的）、enabled。它决定消费什么，不决定怎么处理。

Subscription 的标识里带过滤版本，形如 `checkpoints/<consumer>/<平台>/<类型>/<范围>/<版本>.json`。改过滤条件必须换版本：被过滤掉的数据在模型里算「已消费」，沿用旧标识改过滤，历史数据就永久漏掉，而且看不出来漏了。

### Consumer

Consumer 是独立处理者的身份，同时是 Checkpoint 的所有者。一个 Consumer 可以订阅多个 Source，各自一条 Subscription、各自一份进度；同一个 Source 也可以被多个 Consumer 订阅。Consumer 还要声明自己的幂等策略，见后文。

第一阶段不建消费者注册表：Subscription 由配置声明，Consumer 由代码实现，两边名字对上即可。多一层注册只会多一处需要同步的真相。

### Checkpoint

Checkpoint 是某个 Consumer 在某个 Subscription 上确认处理完的位置，概念上写作 `Checkpoint(Consumer, Subscription) = Position`，落在 `checkpoints/<consumer>/<平台>/<类型>/<范围>/<版本>.json`。

## Position、读位与 State

这三个词在本模型里各指一件事，混用就会丢掉失败恢复的判据。

Position 是数据在来源里的一个点，结构化的，不是一个字符串：

```json
{ "source": "feishu:im:oc_xxx", "by": "create_time", "value": "1712345678000", "tie": "om_abc" }
```

`by` 是 Source 声明的顺序键，`value` 是主值，`tie` 是同值时定次序的次值（通常是 external id）。同一 Source 内可比、可持久化、可换回一次查询；跨 Source 不可比，比较前先校验 `source` 相同。形状固定成三项以后，时间戳、自增号、复合主键都能表达，不必为每种形式写一个类型。来源只能按另一个字段查询时，由适配器把 Position 换成查询下界、再在本地按 Position 截断，转换规则写在 Source 里，不外溢到消费侧。

读位是摄取这一侧物理读到哪，落在 `cursor/<平台>/<类型>/<范围>.json`。它是传输状态，可以超前于任何 Checkpoint，不能当作消费证明。分页 token 属于读位，不属于 Position：飞书的 `page_token` 会过期、随接口变动，把它当进度会让重放依赖一个不可复现的量。旧布局 `cursor/<资源>.json` 不动，已接模型的来源改按新路径读写。

Checkpoint 才是权威的落点。它只由一次处理成功之后写入，且不能比已有的更小。State 是 Consumer 的领域累计结果（`flow.rs` 里「谁在哪一格」就是），可以从日志重放重建，不是位置。

所以「Checkpoint、Cursor、State 哪个更合适」的答案是三个都要，但各管一件事。用 State 代替 Checkpoint 会在失败恢复时没有判据：State 每次处理都会变，变到一半失败时看不出哪些副作用已经发生；用 Cursor 代替 Checkpoint 会把「读过」当成「处理过」，正好是实验版本要解决的问题。

## 摄取与分发

摄取只依赖 Source 与 Store，不依赖 Consumer。一轮摄取做三件事：从读位开始按 Source 声明的扫描方式取页，页循环直接复用 `resources::pull`；把每页原样写进 `raw/`，写进的就是飞书回的那份信封；把页里的记录归一化成 Observation，按位置写进日志。

日志一文件一条，路径由位置决定：`log/<平台>/<类型>/<范围>/<value>/<external_id>.json`。同一条记录被重新观察时覆盖同一个文件，重复摄取不会让文件变多，所以重新观察不产生新位置——要看修订就重放，这条边界在飞书那一节具体写。日志是 `raw/` 的索引，原则上能从 `raw/` 重建，但重建要扫全部留档，成本不低，因此与 `raw/` 一起放 data 目录，不放 cache。

消费只读日志。Consumer 从自己的 committed 位置往后取一段，读到的条数就是这一批；它不看摄取侧的读位，也从不直接调用飞书接口去翻消息。新增一个 Consumer 不需要重新拉数据，从头重放即可，重放的长度等于留档长度。

消费侧仍可能需要直接问来源一些「快照型」的事，例如群成员名单、附件字节、单条消息的详情。这些是按需查询而不是流，不受 Position 与 Checkpoint 管辖，但必须明说：它们的结果不进日志，也不推进任何位置。

留档只增不改；日志按位置一文件一条，同一条记录重看时覆盖同一文件。裁剪是运维动作不是自动清理，且裁剪位置不得高于任何一个 Consumer 的 committed——高过之后那些 Consumer 永远补不回来。

## 提交协议

一次消费分读取、暂存、处理、提交四步，失败就不提交。

```text
读日志(committed 之后的一批)
  ↓
暂存 pending（原子写，记本批起止与 raw 页引用）
  ↓
逐条处理
  ├── 全部成功 → committed := pending.to，清 pending
  └── 任一条失败 → 清 pending，committed 不动
```

pending 与 committed 写在同一个文件里，一次原子替换落盘（写临时文件再 rename）。pending 的用途是让崩溃可辨认，不是让半批可以续传：重启时 pending 非空即视为未提交、直接丢弃，从 committed 重放。这是至少一次语义，第一条不变式要求如此。

提交有两个容易写错的地方。批次的上界取本批扫描到的最大位置，不是最后一条命中过滤的位置，否则尾部连续不命中时每轮重扫同一段。提交只接受严格大于或等于 committed 的位置，相等视为重复提交直接返回成功，小于说明调用序错乱，报错而不是静默回退。

文件内容大致这样：

```json
{
  "consumer": "intent",
  "subscription": "feishu:im:oc_xxx/v1",
  "committed": { "source": "feishu:im:oc_xxx", "by": "create_time", "value": "1712345678000", "tie": "om_abc" },
  "pending": {
    "from": { "source": "feishu:im:oc_xxx", "by": "create_time", "value": "1712345678000", "tie": "om_abc" },
    "to":   { "source": "feishu:im:oc_xxx", "by": "create_time", "value": "1712345800000", "tie": "om_def" },
    "count": 12,
    "raw": ["raw/im/chat-messages/0007.json"]
  },
  "committed_at": "2026-10-23T10:21:02Z"
}
```

`raw` 这一项是证据链：任何一个 Checkpoint 都能指回当时读到的那几页原样响应。

## 幂等

模型只保证至少一次，不保证不重复。重复不会消失，只能被吸收，所以每个 Consumer 必须选一种幂等策略并写在 Subscription 旁边。

自然键覆盖写：业务状态本身以 external id 或人为主键，同一 observation 重放几次结果一样。`flow.rs` 的状态表已经是这个形状。先查后写：对外部系统的写操作先读现状再决定，例如邮件已读与归档的 modify 天然幂等。副作用带标记：往外发消息时带上可反查的标识，重放时先查有没有发过。飞书的发送与回复接口自带幂等键（`+messages-send` / `+messages-reply`），外发优先用它，不另建标记。

模型不建逐条去重表。去重表要按留档长度清理，而留档保留多久是业务决定；建在模型里等于替业务做决定，还多一份必须与留档同步的状态。

重放的确定性由两条保证：日志按位置一文件一条，同一条 observation 重放时字段不变；重新观察只覆盖同一位置的文件，不移动已提交的位置。因此从哪里开始重放、会看到哪些条目都是确定的。代价是编辑不被增量消费看见，要看修订就重放，见下一节。

## 与 engine、store、flow 的接入

`engine.rs` 的 `增量` 声明保留，但收窄为纯函数：按声明的顺序键截断这一批、算出这批的新位置。持久化仍然不在引擎里——引擎现在就不落盘，正好。上下文里的保留名 `游标` 换成结构化的 `position`，`Outcome.cursor` 改成 `Outcome.position`，形状与 Position 一致。这样引擎只回答「这批的新位置是什么」，谁存、何时算数由 Checkpoint 决定。

`store.rs` 加两个路径与两对读写：`log/` 按位置落 observation、`checkpoints/` 落提交文件；`raw/` 与 `resources/` 一个字节都不动。`cursor/` 目录保留，语义收窄为读位，不再承载消费进度；`resources::pull` 的页循环可原样复用，但它现在按资源名推游标，接模型后改成按来源推读位。

`flow.rs` 最后改。现在 `derive` 每轮直接调定义、定义直接打接口，等于每个调用方各自拉一遍。接上模型以后，收集节变成「声明订阅哪些 Source」，`derive` 读日志并走 Checkpoint。这一步放最后，是因为它要同时动流程格式与样例。

## 飞书第一阶段

第一个 Source 是飞书群消息。映射关系如下。

| 实验版 | 飞书对象 | 正式模型 | 职责 |
|:--|:--|:--|:--|
| Chat | 群（`chat_id`） | Source | 声明顺序键、扫描方式、归一化 |
| Poll / watch | `im +chat-messages-list` 翻页 | 摄取 | 一次扫描，写 raw 与日志 |
| Message | 消息（`message_id`） | Observation | 保留 id、时间、发送人、原链接、原页引用 |
| Target | 一个处理者 + 它的筛选 | Subscription | 决定消费哪些消息 |
| Target 的处理代码 | — | Consumer | 处理逻辑加幂等策略 |
| Target × Chat cursor | — | Checkpoint | committed 与 pending |
| Cursor | `page_token` | 读位 | 只用于续读，会过期，不进 Position |

顺序键用 `create_time`。lark-cli 的 `+chat-messages-list` 只有一条排序轴——创建时间，`--order asc` 与 `--start` / `--end` 都按它，接口没有按更新时间查询的入口。这是「顺序键由来源能查询什么决定」的直接结果，不是将就。邮件用 `internal_date`（邮件不可变），GitHub issue 用 `updated_at`（`since` 正是按它筛）。

增量读法是 `--start` 加 `--order asc`：从 committed 的秒级时间往前留一点重叠开始翻页（`--page-all` 或自管 `--page-token` 循环），再在本地按 Position 截断。重叠只用来吸收同一秒内的乱序与延迟写入，长度写在 Source 的声明里，重复部分由幂等吸收。只有来源连范围查询都没有时，才退化成「取最近一段窗口再本地截断」，两种扫描方式都在 Source 里声明。现有的 `im_chat_messages` 登记只有 `--order` 与 `--page-token` 循环，没有时间范围，接模型时补 `start` / `end` 两个参数，照登记即契约的规矩只改 `resources/im.rs` 一处。

编辑与撤回改变 `update_time`，不改变创建时间，因此不产生新位置，重新观察只覆盖同一位置的日志文件。这带来一个明确边界：增量消费看不见已提交位置的修订，要看修订就从更早的位置重放。撤回的消息不删，留在日志与留档里——证据链优先于「当前态」；接口给得出删除标记，就更新在同一位置的文件里。

证据链到 Observation 为止不再加工：`raw_ref` 指向当时那页原样响应，`url` 给一条能回到平台原文的链接，`payload` 只放归一化后的正文与元数据。要还原上下文时回到 `raw/`，不在日志里复制第二份原文。

## 验收问题自答

1. 一个 Source 如何产生 Observation：由 Source 适配器按位置扫描、取页、归一化，摄取侧调一次并写进日志，Consumer 不参与。
2. 一个 Source 如何被多个 Consumer 独立消费：日志共享，按位置一文件一条，各 Consumer 按自己的 Checkpoint 从日志取，不写对方的状态，也不推进读位。
3. 一个 Consumer 如何表达订阅范围：一条 Subscription 写明 source、filter、scope、enabled，标识里带过滤版本，改过滤即换版本、从头重放。
4. 消费位置如何表达：Position 是结构化的 (source, by, value, tie)，同一 Source 内可比较、可持久化；分页 token 归读位，不进 Position。
5. 什么情况下一个 Position 可以成为 Checkpoint：一整批 observation 处理成功，且新位置不小于已提交位置；被过滤掉的条目不阻塞提交。
6. 消费失败后从哪里恢复：从 committed 位置重放，pending 视为未提交，进程重启即丢弃。
7. 如何避免重复消费造成重复副作用：至少一次加每个 Consumer 声明的幂等策略（自然键覆盖、先查后写、副作用带标记），模型不做逐条去重。
8. 如何做到一个 Source 只进行一次物理读取：摄取与消费分离，摄取每轮只读一次并落日志，消费只读本地。
9. 飞书的 Message、Chat、Target 如何映射：Chat 是 Source，Message 是 Observation，Target 是 Subscription，实验版的 cursor 是 Checkpoint，`page_token` 只是读位。
10. 将来加 GitHub、Email 时哪些可以复用：五概念链、Position 形状、提交协议、日志与留档规则全部复用；只需新写 Source 适配器（顺序键、扫描方式、归一化）和一个 Consumer 的幂等策略。

## 非目标与落地顺序

本模型不做的事，与 issue 一致：不直接迁移实验版的 `watch/`，不实现完整消息队列，不引入 Kafka 一类外部基础设施，不一次做完所有 Source，不重构现有的 Resource / Backend / Engine 架构。编辑与撤回的增量处理也不在第一阶段：它们不产生新位置，日志覆盖同位置的文件，重放看得见、增量看不见。

落地分七步，前一步不做完，后一步落不了地。

1. 术语与路径定稿，即本文件；判据是本文件通过评审，`docs/index.md` 链上。
2. `store.rs` 加日志与 Checkpoint 的路径和读写，含原子替换；判据是 `tests/store.rs` 补断言，覆盖原子替换与覆盖写不增文件。
3. 飞书群消息的 Source 适配器：顺序键、增量扫描（`--start`，并留窗口退化的位置）、归一化；判据是假 lark-cli 回夹具，断言 Observation 字段与 Position。
4. 摄取：一次扫描写 `raw/` 与日志；判据是同一次扫描重复跑，日志文件数不变。
5. Consumer 与提交协议；判据是四条测试——成功提交、失败不提交、崩溃丢 pending、乱序提交被拒。
6. 两个 Consumer 共用一次摄取；判据是假 lark-cli 只被调一轮，两份 Checkpoint 各自前进。
7. 引擎的增量接上结构化 Position，`flow.rs` 收集改读日志；判据是 `cargo test --all-targets` 全绿，且 `cargo run --example qtrecurit` 输出与改动前一致。

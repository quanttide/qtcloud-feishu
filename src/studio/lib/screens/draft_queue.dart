import 'package:flutter/material.dart';

import '../services/draft_source.dart';

/// 待确认队列：CLI 落盘的草稿在这里等人确认。
///
/// 一页对应落盘契约里的一种记录：草稿（发信默认起草）。
/// 人只做两件事——确认或驳回；确认的写回方式随 CLI 的草稿确认机制
/// （ROADMAP 阶段三）定形，判据已定：Studio 不发明第二种确认语义。
///
/// 数据源见 [DraftSource]：桌面形态直读 CLI 的 XDG 目录。
class DraftQueueScreen extends StatefulWidget {
  const DraftQueueScreen({super.key, this.source});

  final DraftSource? source;

  @override
  State<DraftQueueScreen> createState() => _DraftQueueScreenState();
}

class _DraftQueueScreenState extends State<DraftQueueScreen> {
  late final Future<DraftLoad> _loading;

  @override
  void initState() {
    super.initState();
    _loading = Future(() async {
      try {
        return await (widget.source ?? XdgDraftSource()).load();
      } catch (e) {
        // web 形态 dart:io 不可用，或目录读不出来：都归为没接上，原因原样带出
        return DraftLoad(available: false, problems: ['读取失败：$e']);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        backgroundColor: Theme.of(context).colorScheme.inversePrimary,
        title: const Text('待确认'),
      ),
      body: FutureBuilder<DraftLoad>(
        future: _loading,
        builder: (context, snapshot) {
          if (snapshot.connectionState != ConnectionState.done) {
            return const Center(child: CircularProgressIndicator());
          }
          final load = snapshot.data;
          if (load == null || snapshot.hasError) {
            return _Problem('数据源读取失败：${snapshot.error}');
          }
          if (!load.available) {
            // 分清两种空：数据源没接上 ≠ 没有草稿
            return _Problem(
                '数据源没接上：${load.problems.join('；')}\n跑过一次起草流程后再来。');
          }
          if (load.drafts.isEmpty) {
            // 没草稿可列时，问题才占整页；有草稿就横幅提示，不顶掉列表
            if (load.problems.isNotEmpty) {
              return _Problem(load.problems.join('\n'));
            }
            return const Center(child: Text('没有待确认的草稿'));
          }
          return ListView(
            children: [
              if (load.problems.isNotEmpty)
                _ProblemBanner(load.problems.join('\n')),
              for (final draft in load.drafts) _DraftCard(draft: draft),
            ],
          );
        },
      ),
    );
  }
}

class _Problem extends StatelessWidget {
  const _Problem(this.message);

  final String message;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Text(message, textAlign: TextAlign.center),
      ),
    );
  }
}

/// 有问题但草稿还在：横幅提示，列表照常显示。
class _ProblemBanner extends StatelessWidget {
  const _ProblemBanner(this.message);

  final String message;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.fromLTRB(12, 12, 12, 0),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.errorContainer,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Text(
        message,
        style: TextStyle(color: Theme.of(context).colorScheme.onErrorContainer),
      ),
    );
  }
}

class _DraftCard extends StatelessWidget {
  const _DraftCard({required this.draft});

  final PendingDraft draft;

  @override
  Widget build(BuildContext context) {
    final recorded = draft.recordedAt;
    return Card(
      margin: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              draft.subject.isEmpty
                  ? draft.draftId
                  : '${draft.subject} · ${draft.to}',
              style: Theme.of(context).textTheme.titleSmall,
            ),
            const SizedBox(height: 4),
            Text(draft.body),
            if (draft.missing.isNotEmpty) ...[
              const SizedBox(height: 4),
              Text(
                '信封里缺这些字段：${draft.missing.join('、')}（以落盘为准，不猜）',
                style: Theme.of(context)
                    .textTheme
                    .bodySmall
                    ?.copyWith(color: Theme.of(context).colorScheme.error),
              ),
            ],
            const SizedBox(height: 8),
            Row(
              children: [
                Text(
                  recorded == null
                      ? draft.candidate
                      : '${recorded.year}-${recorded.month.toString().padLeft(2, '0')}-${recorded.day.toString().padLeft(2, '0')} 落盘 · ${draft.candidate}',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
                const Spacer(),
                // 写回随 CLI 阶段三定形后启用；语义跟 CLI 一种，不在这发明。
                FilledButton(onPressed: null, child: const Text('确认发送')),
                const SizedBox(width: 8),
                OutlinedButton(onPressed: null, child: const Text('驳回')),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

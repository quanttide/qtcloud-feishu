import 'package:flutter/material.dart';

/// 待确认队列：CLI 落盘的草稿在这里等人确认。
///
/// 一页对应落盘契约里的一种记录：草稿（如发信默认草稿）。
/// 人只做两件事——确认或驳回；确认后由执行器真正写回，
/// 本页不直接调 lark-cli，也不自己发信。
///
/// 数据源还没接：CLI 的草稿动作在迁入清单第 5 步（邮件发送通道），
/// 落盘记录出现后把 [Draft] 字段与目录对齐，这里从落盘目录读。
class DraftQueueScreen extends StatelessWidget {
  const DraftQueueScreen({super.key, this.drafts = const []});

  final List<Draft> drafts;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        backgroundColor: Theme.of(context).colorScheme.inversePrimary,
        title: const Text('待确认'),
      ),
      body: drafts.isEmpty
          ? const Center(child: Text('没有待确认的草稿'))
          : ListView.builder(
              itemCount: drafts.length,
              itemBuilder: (context, i) => _DraftCard(draft: drafts[i]),
            ),
    );
  }
}

/// 一条草稿。字段先按 CLI 落盘契约的最小面立：
/// 动作、收件人、正文、落盘时间。落盘记录出现后如有出入，以落盘为准。
class Draft {
  const Draft({
    required this.action,
    required this.recipient,
    required this.body,
    required this.recordedAt,
  });

  final String action; // 所属动作，如 mail_send
  final String recipient;
  final String body;
  final DateTime recordedAt;
}

class _DraftCard extends StatelessWidget {
  const _DraftCard({required this.draft});

  final Draft draft;

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('${draft.action} · ${draft.recipient}',
                style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 4),
            Text(draft.body),
            const SizedBox(height: 8),
            Row(
              children: [
                Text(
                  '${draft.recordedAt.year}-${draft.recordedAt.month.toString().padLeft(2, '0')}-${draft.recordedAt.day.toString().padLeft(2, '0')} 落盘',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
                const Spacer(),
                // 驳回先不做：确认写回随 CLI 的草稿确认机制（roadmap 阶段二）定形；
                // 判据已定——Studio 不发明第二种确认语义，CLI 定成动作就走动作，定成状态就改状态。
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

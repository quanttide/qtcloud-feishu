import 'dart:convert';
import 'dart:io';

/// 待确认草稿的数据源：直读 CLI 的 XDG 落盘目录（桌面形态）。
///
/// 契约（CLI ROADMAP 阶段二，2026-10-29）：
/// - 待确认 = state 文件（`<state>/qtcloud-feishu/<流程名>.json`）里 `confirmed` 为空
///   且 `artifacts.草稿id` 非空的条目
/// - 草稿内容 = 信封（`raw/mail/draft-create/<页序>.json`）优先；信封只回草稿 id 时，
///   收件人与主题从 `ops.jsonl` 的操作日志兜底（同一份 CLI 契约的另一处落盘）；
///   正文按隐私规则不进日志，信封没有就报缺——平台字段名会变，以落盘为准，不猜
/// - 确认写回不在这：那是 ROADMAP 阶段三（与 CLI 的 `flow::confirm` 同一语义）
///
/// web 静态产物读不了本地目录，这条通路只在桌面形态成立；上云时换成服务封装，
/// 本类的对外形状（`DraftLoad`）不变。
class PendingDraft {
  const PendingDraft({
    required this.draftId,
    required this.candidate,
    required this.to,
    required this.subject,
    required this.body,
    required this.recordedAt,
    this.missing = const [],
  });

  final String draftId;
  final String candidate; // 状态表里的键：候选人邮箱
  final String to;
  final String subject;
  final String body;
  final DateTime? recordedAt;
  final List<String> missing; // 信封里没找到的字段名——报出来，不猜
}

class DraftLoad {
  const DraftLoad({
    required this.available,
    this.drafts = const [],
    this.problems = const [],
  });

  /// 数据目录在不在。false ＝ 数据源没接上（不是「没有草稿」）。
  final bool available;
  final List<PendingDraft> drafts;
  final List<String> problems; // 读不出来的文件、对不上的草稿——原样报
}

abstract class DraftSource {
  Future<DraftLoad> load();
}

class XdgDraftSource implements DraftSource {
  XdgDraftSource({Directory? stateDir, Directory? dataDir})
      : stateDir = stateDir ?? _xdgDir('XDG_STATE_HOME', ['.local', 'state']),
        dataDir = dataDir ?? _xdgDir('XDG_DATA_HOME', ['.local', 'share']);

  /// `<state>/qtcloud-feishu`：流程状态文件在这。
  final Directory stateDir;

  /// `<data>/qtcloud-feishu`：raw/ 镜像在这。
  final Directory dataDir;

  static Directory _xdgDir(String env, List<String> fallback) {
    final value = Platform.environment[env];
    if (value != null && value.isNotEmpty && value.startsWith('/')) {
      return Directory('$value/qtcloud-feishu');
    }
    final home = Platform.environment['HOME'] ?? '';
    return Directory('$home/${fallback.join('/')}/qtcloud-feishu');
  }

  @override
  Future<DraftLoad> load() async {
    if (!stateDir.existsSync()) {
      return DraftLoad(
        available: false,
        problems: ['状态目录不存在：${stateDir.path}（流程还没跑过？）'],
      );
    }
    final problems = <String>[];
    final pending = <String, String>{}; // 候选人 → 草稿 id

    for (final file in _jsonFiles(stateDir)) {
      try {
        final state = jsonDecode(await file.readAsString());
        if (state is! Map<String, dynamic>) continue;
        state.forEach((candidate, value) {
          if (value is! Map<String, dynamic>) return;
          final confirmed = (value['confirmed'] as String?) ?? '';
          final artifacts = value['artifacts'];
          final draftId = artifacts is Map<String, dynamic>
              ? artifacts['草稿id'] as String?
              : null;
          if (confirmed.isEmpty && draftId != null && draftId.isNotEmpty) {
            pending[candidate] = draftId;
          }
        });
      } catch (e) {
        problems.add('状态文件读不出来 ${file.path}：$e');
      }
    }

    // 信封按 draft_id 对上；字段以落盘为准，缺哪个记哪个
    final envelopes = <String, _Envelope>{};
    final rawDir = Directory('${dataDir.path}/raw/mail/draft-create');
    if (rawDir.existsSync()) {
      for (final file in _jsonFiles(rawDir)) {
        try {
          final envelope = jsonDecode(await file.readAsString());
          if (envelope is! Map<String, dynamic>) continue;
          final data = envelope['data'];
          if (data is! Map<String, dynamic>) continue;
          final id = data['draft_id'] as String?;
          if (id != null && id.isNotEmpty) {
            envelopes[id] = _Envelope(data, file.lastModifiedSync());
          }
        } catch (e) {
          problems.add('草稿信封读不出来 ${file.path}：$e');
        }
      }
    }

    // 操作日志：起草动作记了 to/subject 与草稿 id，信封缺字段时从这里兑底
    final logged = <String, Map<String, dynamic>>{};
    final opsFile = File('${stateDir.path}/ops.jsonl');
    if (opsFile.existsSync()) {
      for (final line in await opsFile.readAsLines()) {
        if (line.trim().isEmpty) continue;
        try {
          final entry = jsonDecode(line);
          if (entry is! Map<String, dynamic>) continue;
          if (entry['action'] != 'mail_draft_create') continue;
          final id = entry['id'] as String?;
          final params = entry['params'];
          if (id != null && params is Map<String, dynamic>) {
            logged[id] = params;
          }
        } catch (e) {
          problems.add('操作日志有一行读不出来：$e');
        }
      }
    }

    final drafts = <PendingDraft>[];
    pending.forEach((candidate, draftId) {
      final envelope = envelopes[draftId];
      if (envelope == null) {
        problems.add('草稿 $draftId（$candidate）在 raw/mail/draft-create/ 里没有对应信封');
        drafts.add(PendingDraft(
          draftId: draftId,
          candidate: candidate,
          to: '',
          subject: '',
          body: '',
          recordedAt: null,
          missing: ['信封'],
        ));
        return;
      }
      final missing = <String>[];
      final ops = logged[draftId] ?? const <String, dynamic>{};
      String pick(String key) {
        final fromEnvelope = envelope.data[key];
        if (fromEnvelope is String && fromEnvelope.isNotEmpty) {
          return fromEnvelope;
        }
        final fromOps = ops[key];
        if (fromOps is String && fromOps.isNotEmpty) {
          return fromOps;
        }
        missing.add(key);
        return '';
      }

      drafts.add(PendingDraft(
        draftId: draftId,
        candidate: candidate,
        to: pick('to'),
        subject: pick('subject'),
        body: pick('body'),
        recordedAt: envelope.recordedAt,
        missing: missing,
      ));
    });
    return DraftLoad(available: true, drafts: drafts, problems: problems);
  }

  Iterable<File> _jsonFiles(Directory dir) => dir
      .listSync()
      .whereType<File>()
      .where((file) => file.path.endsWith('.json'));
}

class _Envelope {
  const _Envelope(this.data, this.recordedAt);

  final Map<String, dynamic> data;
  final DateTime recordedAt;
}

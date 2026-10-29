import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

import 'package:qtcloud_feishu_studio/services/draft_source.dart';

void main() {
  late Directory root;

  setUp(() {
    root = Directory.systemTemp.createTempSync('draft-source-test');
  });

  tearDown(() {
    root.deleteSync(recursive: true);
  });

  Future<DraftLoad>? pendingLoad;

  /// 造一套最小落盘：state 文件 + 草稿信封。
  void writeLanding({
    String confirmed = '',
    bool withDraftId = true,
    bool withEnvelope = true,
    String envelopeBody = '扫码进群',
  }) {
    final stateDir = Directory('${root.path}/state/qtcloud-feishu')
      ..createSync(recursive: true);
    final entry = {
      'since': '2026-10-13',
      'state': '待确认',
      'trail': ['投递', '待确认'],
      if (withDraftId)
        'artifacts': {
          '草稿id': 'mock-draft-001',
        },
      'confirmed': confirmed,
    };
    File('${stateDir.path}/招聘.json')
        .writeAsStringSync(jsonEncode({'chenmo@example.com': entry}));

    final dataDir = Directory('${root.path}/data/qtcloud-feishu')
      ..createSync(recursive: true);
    if (withEnvelope) {
      final rawDir = Directory('${dataDir.path}/raw/mail/draft-create')
        ..createSync(recursive: true);
      File('${rawDir.path}/0001.json').writeAsStringSync(jsonEncode({
        'ok': true,
        'data': {
          'draft_id': 'mock-draft-001',
          'to': 'chenmo@example.com',
          'subject': '量潮实训基地 · 群二维码',
          'body': envelopeBody,
        },
      }));
    }
    pendingLoad = XdgDraftSource(stateDir: stateDir, dataDir: dataDir).load();
  }

  test('状态目录不在就报数据源没接上，不是空队列', () async {
    final source = XdgDraftSource(
      stateDir: Directory('${root.path}/absent-state'),
      dataDir: Directory('${root.path}/absent-data'),
    );
    final load = await source.load();
    expect(load.available, isFalse);
    expect(load.drafts, isEmpty);
    expect(load.problems.single, contains('状态目录不存在'));
  });

  test('confirmed 为空且有草稿 id 才算待确认', () async {
    writeLanding();
    final load = await pendingLoad!;
    expect(load.available, isTrue);
    expect(load.drafts, hasLength(1));
    final draft = load.drafts.single;
    expect(draft.draftId, 'mock-draft-001');
    expect(draft.candidate, 'chenmo@example.com');
    expect(draft.to, 'chenmo@example.com');
    expect(draft.subject, '量潮实训基地 · 群二维码');
    expect(draft.body, '扫码进群');
    expect(draft.recordedAt, isNotNull, reason: '落盘时间取信封文件的修改时间');
    expect(draft.missing, isEmpty);
    expect(load.problems, isEmpty);
  });

  test('确认过的不再进队列', () async {
    writeLanding(confirmed: '2026-10-14');
    final load = await pendingLoad!;
    expect(load.drafts, isEmpty);
  });

  test('草稿 id 没有对应信封要报出来，不猜', () async {
    writeLanding(withEnvelope: false);
    final load = await pendingLoad!;
    expect(load.drafts, hasLength(1), reason: '条目还在，只是内容缺');
    expect(load.drafts.single.missing, ['信封']);
    expect(load.problems.single, contains('没有对应信封'));
  });

  test('信封缺字段逐个记下', () async {
    final stateDir = Directory('${root.path}/state/qtcloud-feishu')
      ..createSync(recursive: true);
    File('${stateDir.path}/招聘.json').writeAsStringSync(jsonEncode({
      'chenmo@example.com': {
        'since': '2026-10-13',
        'state': '待确认',
        'trail': ['投递'],
        'artifacts': {'草稿id': 'mock-draft-001'},
        'confirmed': '',
      },
    }));
    final dataDir = Directory('${root.path}/data/qtcloud-feishu')
      ..createSync(recursive: true);
    final rawDir = Directory('${dataDir.path}/raw/mail/draft-create')
      ..createSync(recursive: true);
    File('${rawDir.path}/0001.json').writeAsStringSync(jsonEncode({
      'ok': true,
      'data': {'draft_id': 'mock-draft-001', 'subject': '只有主题'},
    }));
    final load =
        await XdgDraftSource(stateDir: stateDir, dataDir: dataDir).load();
    expect(load.drafts.single.missing, containsAll(['to', 'body']));
    expect(load.drafts.single.subject, '只有主题');
  });

  test('信封只回草稿 id 时收件人与主题从操作日志兑底', () async {
    final stateDir = Directory('${root.path}/state/qtcloud-feishu')
      ..createSync(recursive: true);
    File('${stateDir.path}/招聘.json').writeAsStringSync(jsonEncode({
      'chenmo@example.com': {
        'since': '2026-10-13',
        'state': '待确认',
        'trail': ['投递'],
        'artifacts': {'草稿id': 'mock-draft-001'},
        'confirmed': '',
      },
    }));
    // 操作日志：起草动作成功后追加的一行（照 CLI 的 ops.jsonl 形状）
    File('${stateDir.path}/ops.jsonl').writeAsStringSync([
      jsonEncode({
        'time': '2026-10-13T10:00:00',
        'action': 'mail_draft_create',
        'resource': 'mail',
        'params': {
          'to': 'chenmo@example.com',
          'subject': '面试邀请',
        },
        'id': 'mock-draft-001',
        'status': 'ok',
      }),
      '',
    ].join('\n'));
    final dataDir = Directory('${root.path}/data/qtcloud-feishu')
      ..createSync(recursive: true);
    final rawDir = Directory('${dataDir.path}/raw/mail/draft-create')
      ..createSync(recursive: true);
    File('${rawDir.path}/0001.json').writeAsStringSync(jsonEncode({
      'ok': true,
      'data': {'draft_id': 'mock-draft-001'},
    }));
    final load =
        await XdgDraftSource(stateDir: stateDir, dataDir: dataDir).load();
    final draft = load.drafts.single;
    expect(draft.to, 'chenmo@example.com', reason: '从操作日志兑底');
    expect(draft.subject, '面试邀请');
    expect(draft.missing, ['body'], reason: '正文按隐私规则不进日志，信封没有就报缺');
  });

  test('状态文件是坏 JSON 记进问题清单，不影响别的条目', () async {
    final stateDir = Directory('${root.path}/state/qtcloud-feishu')
      ..createSync(recursive: true);
    File('${stateDir.path}/坏文件.json').writeAsStringSync('{不是 json');
    File('${stateDir.path}/招聘.json').writeAsStringSync(jsonEncode({
      'herui@example.com': {
        'since': '2026-10-13',
        'state': '待确认',
        'trail': ['投递'],
        'artifacts': {'草稿id': 'mock-draft-002'},
        'confirmed': '',
      },
    }));
    final dataDir = Directory('${root.path}/data/qtcloud-feishu')
      ..createSync(recursive: true);
    final rawDir = Directory('${dataDir.path}/raw/mail/draft-create')
      ..createSync(recursive: true);
    File('${rawDir.path}/0001.json').writeAsStringSync(jsonEncode({
      'ok': true,
      'data': {
        'draft_id': 'mock-draft-002',
        'to': 'herui@example.com',
        'subject': '主题',
        'body': '正文',
      },
    }));
    final load =
        await XdgDraftSource(stateDir: stateDir, dataDir: dataDir).load();
    expect(load.problems.single, contains('坏文件.json'));
    expect(load.drafts, hasLength(1), reason: '坏文件不拖累好的');
  });
}

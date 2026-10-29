import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:qtcloud_feishu_studio/main.dart';
import 'package:qtcloud_feishu_studio/services/draft_source.dart';

class _FakeSource implements DraftSource {
  _FakeSource(this.result);

  final DraftLoad result;

  @override
  Future<DraftLoad> load() async => result;
}

class _ThrowingSource implements DraftSource {
  @override
  Future<DraftLoad> load() async => throw UnsupportedError('web 读不了本地目录');
}

void main() {
  testWidgets('数据源没接上时说清楚是没接上，不是没草稿', (tester) async {
    await tester.pumpWidget(StudioApp(
        source: _FakeSource(const DraftLoad(
            available: false, problems: ['状态目录不存在：/nowhere']))));
    await tester.pumpAndSettle();

    expect(find.text('待确认'), findsOneWidget);
    expect(find.textContaining('数据源没接上'), findsOneWidget);
    expect(find.text('没有待确认的草稿'), findsNothing);
  });

  testWidgets('数据源抛错按没接上呈现，不白屏', (tester) async {
    await tester.pumpWidget(StudioApp(source: _ThrowingSource()));
    await tester.pumpAndSettle();

    expect(find.textContaining('数据源没接上'), findsOneWidget);
  });

  testWidgets('接上了但队列为空显示没有草稿', (tester) async {
    await tester.pumpWidget(
        StudioApp(source: _FakeSource(const DraftLoad(available: true))));
    await tester.pumpAndSettle();

    expect(find.text('没有待确认的草稿'), findsOneWidget);
  });

  testWidgets('有草稿时显示卡片、字段缺失要报出来、按钮仍禁用',
      (tester) async {
    const draft = PendingDraft(
      draftId: 'mock-draft-001',
      candidate: 'chenmo@example.com',
      to: 'chenmo@example.com',
      subject: '面试邀请',
      body: '正文见附件',
      recordedAt: null,
      missing: ['body'],
    );
    await tester.pumpWidget(StudioApp(
        source: _FakeSource(const DraftLoad(available: true, drafts: [draft]))));
    await tester.pumpAndSettle();

    expect(find.text('面试邀请 · chenmo@example.com'), findsOneWidget);
    expect(find.text('正文见附件'), findsOneWidget);
    expect(find.textContaining('body'), findsOneWidget, reason: '缺字段要报');
    final confirm =
        tester.widget<FilledButton>(find.widgetWithText(FilledButton, '确认发送'));
    final reject = tester.widget<OutlinedButton>(
        find.widgetWithText(OutlinedButton, '驳回'));
    expect(confirm.onPressed, isNull, reason: '写回是阶段三，先禁用');
    expect(reject.onPressed, isNull);
  });

  testWidgets('读取出错显示问题而不是空白页', (tester) async {
    await tester.pumpWidget(StudioApp(
        source: _FakeSource(const DraftLoad(
            available: true, problems: ['状态文件读不出来 /x/y.json']))));
    await tester.pumpAndSettle();

    expect(find.textContaining('状态文件读不出来'), findsOneWidget);
  });
}

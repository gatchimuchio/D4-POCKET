// 開発専用。owner承認は外側の隔離試験が行い、製品clientは閲覧と新規承認待ち要求までを行う。
import 'package:gui_shell_ui/history_client.dart';
import 'package:gui_shell_desktop/services/broker_client.dart';

Future<void> main(List<String> args) async {
  if (args.length != 2) throw ArgumentError('通常資格fileとRuntimeが必要');
  final transport = await BrokerClient.connect(sessionFile: args[0]);
  final client = HistoryClient(transport);
  final grant = await client.status();
  if (grant == null || grant.runtime != args[1]) throw StateError('現在承認が不一致');
  var after = 0;
  var count = 0;
  var proofs = 0;
  for (var i = 0; i < 32; i++) {
    final page = await client.page(grant, after: after);
    if (count == 0 && page.entries.isNotEmpty) {
      final first = page.entries.first.record.fields;
      final filtered = await client.page(grant,
          requestId: first['要求ID'] as String,
          sessionId: first['対話セッションID'] as String);
      if (filtered.entries.length != 1 ||
          filtered.entries.single.record.fields['要求ID'] != first['要求ID']) {
        throw StateError('実要求とSessionの検索不一致');
      }
      final created = await client
          .replay(grant, page.entries.first, '製品clientの分岐確認', branch: true);
      if (created['状態'] != '承認待ち') throw StateError('新要求が承認待ちではない');
      final closed = await transport
          .request('対話終了', payload: {'対話セッションID': created['対話セッションID']});
      if (closed['status'] != 'accepted') throw StateError('試験要求の終了未成立');
    }
    proofs += page.entries.where((e) => e.evidence != null).length;
    count += page.entries.length;
    if (!page.more) {
      if (count == 0 || proofs == 0) throw StateError('実履歴または結果証跡がない');
      return;
    }
    after = page.next;
  }
  throw StateError('実履歴の取得上限');
}

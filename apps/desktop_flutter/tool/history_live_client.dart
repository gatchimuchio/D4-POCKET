// 開発専用。owner承認は外側の隔離試験が行い、製品clientは閲覧だけを行う。
import 'package:gui_shell_ui/history_client.dart';
import 'package:gui_shell_desktop/services/broker_client.dart';

Future<void> main(List<String> args) async {
  if (args.length != 2) throw ArgumentError('通常資格fileとRuntimeが必要');
  final client =
      HistoryClient(await BrokerClient.connect(sessionFile: args[0]));
  final grant = await client.status();
  if (grant == null || grant.runtime != args[1]) throw StateError('現在承認が不一致');
  var after = 0;
  var count = 0;
  for (var i = 0; i < 32; i++) {
    final page = await client.page(grant, after: after);
    count += page.entries.length;
    if (!page.more) {
      if (count == 0) throw StateError('実履歴がない');
      return;
    }
    after = page.next;
  }
  throw StateError('実履歴の取得上限');
}

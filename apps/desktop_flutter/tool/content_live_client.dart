// 開発専用。外側の隔離試験が承認し、この製品clientは通常資格だけを使う。
import 'package:gui_shell_ui/history_client.dart';
import 'package:gui_shell_desktop/services/broker_client.dart';

Future<void> main(List<String> args) async {
  if (args.length != 2) throw ArgumentError('通常資格fileと要求IDが必要');
  final client =
      HistoryClient(await BrokerClient.connect(sessionFile: args[0]));
  final history = await client.status();
  if (history == null) throw StateError('履歴承認がない');
  final page = await client.page(history, requestId: args[1]);
  final entry = page.entries.single;
  final grant = await client.contentStatus();
  if (grant == null || !grant.matches(entry)) throw StateError('内容承認が不一致');
  final content = await client.content(entry, grant);
  if (content.input != 'こんにちは' ||
      content.result.text('本文').isEmpty ||
      content.result.text('応答hash') != entry.evidence!.responseHash) {
    throw StateError('実保管内容が不一致');
  }
}

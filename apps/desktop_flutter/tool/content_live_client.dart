// 開発専用の旧TCP Broker試験fixture。製品Flutter経路からは使用しない。
import 'package:gui_shell_ui/history_client.dart';
import '../test/support/test_broker_tcp_transport.dart';

Future<void> main(List<String> args) async {
  if (args.length != 2) throw ArgumentError('通常資格fileと要求IDが必要');
  final client = HistoryClient(await TestBrokerTcpTransport.connect(args[0]));
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

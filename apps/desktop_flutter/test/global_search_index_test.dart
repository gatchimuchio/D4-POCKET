import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/models/generated_contracts.dart';
import 'package:gui_shell_desktop/services/global_search_index.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';

void main() {
  test('全体検索はMCPとA2Aを表示専用のsurface結果として見つける', () {
    final snapshot = ShellCoreClient.mock().getSnapshot();

    final mcp = GlobalSearchIndex.search(snapshot, 'MCP');
    final a2a = GlobalSearchIndex.search(snapshot, 'A2A');

    expect(mcp, isNotEmpty);
    expect(mcp.first.title, 'MCP接続');
    expect(mcp.first.pageIndex, 11);
    expect(a2a.first.title, 'A2A接続');
    expect(a2a.first.evidenceSource, '不明');
  });

  test('全体検索はqueryと結果件数をboundedにしraw contentを射影しない', () {
    final snapshot = ShellCoreClient.mock().getSnapshot();

    expect(
      GlobalSearchIndex.search(snapshot, 'a' * 129),
      isEmpty,
    );
    final results = GlobalSearchIndex.search(snapshot, '');
    expect(results.length, lessThanOrEqualTo(GlobalSearchIndex.maxResults));
    for (final result in results) {
      expect(result.detail, isNot(contains('projected_content')));
    }
  });

  test('取込JSONのsnapshot_source自己申告はBroker証拠へ昇格しない', () {
    final snapshot = ShellSnapshot.fromJson({
      'snapshot_source': 'broker',
      'runtimes': [
        {
          'runtime_id': 'forged-runtime',
          'name': '未検証Runtime',
          'status': 'ready',
          'adapter_id': 'unverified-adapter',
        },
      ],
    });

    expect(snapshot.snapshotSource, 'unverified');
    final evidenceSources = GlobalSearchIndex.build(snapshot)
        .map((result) => result.evidenceSource)
        .toSet();
    expect(evidenceSources, {'不明'});
  });
}

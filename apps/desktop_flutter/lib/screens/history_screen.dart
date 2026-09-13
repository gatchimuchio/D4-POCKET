import 'dart:async';
import 'package:flutter/material.dart';
import 'package:gui_shell_ui/gui_shell_ui.dart';
import '../services/broker_client.dart';

class HistoryScreen extends StatefulWidget {
  const HistoryScreen({super.key, this.client});
  final HistoryClient? client;
  @override
  State<HistoryScreen> createState() => _HistoryScreenState();
}

class _HistoryScreenState extends State<HistoryScreen>
    with WidgetsBindingObserver {
  HistoryClient? _client;
  HistoryPage? _page;
  Timer? _timer;
  int _generation = 0;
  bool _busy = false, _active = true;
  String? _state;
  String _message = '履歴閲覧にはownerの期限付き承認が必要です。';
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _timer = Timer.periodic(const Duration(milliseconds: 100), (_) {
      if (mounted && _page != null && !_page!.grant.current) {
        _generation++;
        setState(() {
          _page = null;
          _busy = false;
          _message = '閲覧期限を過ぎました。';
        });
      }
    });
    _load();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    _active = state == AppLifecycleState.resumed;
    _generation++;
    setState(() {
      _page = null;
      _busy = false;
      _message = '画面復帰後は更新してください。';
    });
  }

  @override
  void dispose() {
    _generation++;
    _timer?.cancel();
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  Future<void> _load({int after = 0}) async {
    if (!_active) return;
    final generation = ++_generation;
    setState(() {
      _page = null;
      _busy = true;
      _message = '現在承認と履歴を確認中';
    });
    try {
      _client ??= widget.client ?? HistoryClient(await BrokerClient.connect());
      final grant = await _client!.status();
      if (grant == null) {
        throw const BrokerClientException('ownerの履歴閲覧承認がありません。');
      }
      final page = await _client!.page(grant, after: after, state: _state);
      if (!mounted || !_active || generation != _generation) return;
      setState(() {
        _page = page;
        _message = '要求ごとの最後の観測です。現在の稼働・実行許可を示しません。';
      });
      Future<void>.delayed(const Duration(seconds: 2), () {
        if (mounted && _active && generation == _generation) {
          _load(after: after);
        }
      });
    } catch (_) {
      if (mounted && generation == _generation) {
        setState(() {
          _page = null;
          _message = '現在の履歴閲覧承認または接続を確認できません。';
        });
      }
    } finally {
      if (mounted && generation == _generation) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final page = _page;
    return Padding(
        padding: const EdgeInsets.all(16),
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text('実行履歴', style: Theme.of(context).textTheme.headlineSmall),
          Text(_message),
          Wrap(spacing: 12, children: [
            DropdownButton<String>(
                value: _state ?? 'すべて',
                items: ['すべて', '承認待ち', '実行中', '成功', '保留', '失敗', '中止']
                    .map((s) => DropdownMenuItem(value: s, child: Text(s)))
                    .toList(),
                onChanged: _busy
                    ? null
                    : (s) {
                        _state = s == 'すべて' ? null : s;
                        _load();
                      }),
            TextButton(
                onPressed: _busy ? null : () => _load(),
                child: const Text('先頭から更新')),
            TextButton(
                onPressed: page?.more == true && !_busy
                    ? () => _load(after: page!.next)
                    : null,
                child: const Text('次のページ')),
          ]),
          if (_busy) const LinearProgressIndicator(),
          if (page != null)
            Text(
                'Runtime: ${page.grant.runtime} ／ ${page.entries.length}件の要求'),
          if (page != null && page.entries.isEmpty) const Text('該当する履歴はありません。'),
          Expanded(
              child: ListView.builder(
                  itemCount: page?.entries.length ?? 0,
                  itemBuilder: (context, index) {
                    final e = page!.entries[index];
                    final r = e.record;
                    return ExpansionTile(
                        key: ValueKey(e.auditId),
                        title: Text('${e.state} ／ ${r.fields['要求ID']}'),
                        subtitle: Text('開始 ${r.time('開始時刻')}'),
                        children: [
                          SelectableText(
                              'Session: ${r.fields['対話セッションID']}\n終了: ${r.time('終了時刻')}\n失敗分類: ${e.failure ?? 'なし'}\n履歴監査: ${e.auditId}\n作成監査: ${r.audit('作成監査ID')}\n開始監査: ${r.audit('開始監査ID')}\n終了監査: ${r.audit('終了監査ID')}')
                        ]);
                  })),
        ]));
  }
}

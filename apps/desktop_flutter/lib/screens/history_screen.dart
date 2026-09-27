import 'dart:async';
import 'package:flutter/material.dart';
import 'package:gui_shell_ui/gui_shell_ui.dart';
import '../services/broker_client.dart';
import 'history_content_dialog.dart';

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
  Timer? _expiryTimer, _refreshTimer;
  final _input = TextEditingController();
  final _inputFocus = FocusNode();
  final _requestFilter = TextEditingController();
  final _sessionFilter = TextEditingController();
  String? _requestId, _sessionId;
  final _expanded = <String>{};
  HistoryEntry? _selected;
  Map<String, Object?>? _created;
  void _clearSelection() {
    _expanded.clear();
    _selected = null;
    _created = null;
    _input.clear();
  }

  int _generation = 0;
  bool _busy = false, _active = true;
  String? _state;
  String _message = '履歴閲覧にはownerの期限付き承認が必要です。';
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _load();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    _active = state == AppLifecycleState.resumed;
    _generation++;
    _expiryTimer?.cancel();
    _refreshTimer?.cancel();
    setState(() {
      _page = null;
      _busy = false;
      _clearSelection();
      _message = '画面復帰後は更新してください。';
    });
  }

  @override
  void dispose() {
    _generation++;
    _expiryTimer?.cancel();
    _refreshTimer?.cancel();
    _input.dispose();
    _requestFilter.dispose();
    _sessionFilter.dispose();
    _inputFocus.dispose();
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  Future<void> _load({int after = 0}) async {
    if (!_active) return;
    _expiryTimer?.cancel();
    _refreshTimer?.cancel();
    final generation = ++_generation;
    final previousGrant = _page?.grant;
    final restoreInputFocus = _inputFocus.hasFocus;
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
      final page = await _client!.page(grant,
          after: after,
          state: _state,
          requestId: _requestId,
          sessionId: _sessionId);
      if (!mounted || !_active || generation != _generation) return;
      setState(() {
        if (previousGrant != null && !previousGrant.same(page.grant)) {
          _clearSelection();
        }
        _expanded.retainAll(page.entries.map((e) => e.auditId));
        _page = page;
        _message = '要求ごとの最後の観測です。現在の稼働・実行許可を示しません。';
      });
      _scheduleExpiry(page.grant);
      if (restoreInputFocus && _selected != null) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (mounted &&
              _active &&
              generation == _generation &&
              _page != null &&
              _selected != null) {
            _inputFocus.requestFocus();
          }
        });
      }
      _refreshTimer = Timer(const Duration(seconds: 2), () {
        if (mounted && _active && generation == _generation) {
          _load(after: after);
        }
      });
    } catch (_) {
      if (mounted && generation == _generation) {
        setState(() {
          _page = null;
          _clearSelection();
          _message = '現在の履歴閲覧承認または接続を確認できません。';
        });
      }
    } finally {
      if (mounted && generation == _generation) setState(() => _busy = false);
    }
  }

  void _scheduleExpiry(HistoryGrant grant) {
    _expiryTimer?.cancel();
    if (!mounted || _page == null || !_page!.grant.same(grant)) return;
    _expiryTimer = Timer(grant.untilExpiry, () {
      if (!mounted || _page == null || !_page!.grant.same(grant)) return;
      if (_page!.grant.current) {
        _scheduleExpiry(_page!.grant);
        return;
      }
      _generation++;
      _refreshTimer?.cancel();
      setState(() {
        _page = null;
        _busy = false;
        _clearSelection();
        _message = '閲覧期限を過ぎました。';
      });
    });
  }

  Future<void> _replay(bool branch) async {
    final page = _page;
    final selected = _selected;
    if (_busy || page == null || selected == null) return;
    _refreshTimer?.cancel();
    final generation = ++_generation;
    setState(() {
      _busy = true;
      _created = null;
    });
    try {
      final result = await _client!
          .replay(page.grant, selected, _input.text, branch: branch);
      if (!mounted || !_active || generation != _generation) return;
      setState(() {
        _created = result;
        _selected = null;
        _input.clear();
      });
    } catch (_) {
      if (mounted && generation == _generation) {
        setState(() {
          _created = null;
          _message = '要求を確認できません。再送前に履歴を確認してください。';
        });
      }
    } finally {
      if (mounted && generation == _generation) {
        setState(() => _busy = false);
        _refreshTimer = Timer(const Duration(seconds: 2), () {
          if (mounted && _active && generation == _generation) _load();
        });
      }
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
            SizedBox(
                width: 280,
                child: TextField(
                    controller: _requestFilter,
                    enabled: !_busy,
                    decoration:
                        const InputDecoration(labelText: '要求ID（完全一致）'))),
            SizedBox(
                width: 280,
                child: TextField(
                    controller: _sessionFilter,
                    enabled: !_busy,
                    decoration:
                        const InputDecoration(labelText: 'Session ID（完全一致）'))),
            TextButton(
                onPressed: _busy
                    ? null
                    : () {
                        _requestId = _requestFilter.text.isEmpty
                            ? null
                            : _requestFilter.text;
                        _sessionId = _sessionFilter.text.isEmpty
                            ? null
                            : _sessionFilter.text;
                        _clearSelection();
                        _load();
                      },
                child: const Text('検索')),
            TextButton(
                onPressed: _busy
                    ? null
                    : () {
                        _requestFilter.clear();
                        _sessionFilter.clear();
                        _requestId = null;
                        _sessionId = null;
                        _state = null;
                        _clearSelection();
                        _load();
                      },
                child: const Text('検索条件を解除')),
          ]),
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
          if (page != null && _selected != null) ...[
            Text('参照要求: ${_selected!.record.fields['要求ID']}'),
            const Text('新しいSessionを作成します。再実行は元と同じ入力が必要です。分岐は会話内容を復元しません。'),
            TextField(
                key: const ValueKey('history-replay-input'),
                controller: _input,
                focusNode: _inputFocus,
                enabled: !_busy,
                maxLength: 4096,
                decoration: const InputDecoration(labelText: '新要求の入力')),
            Wrap(children: [
              TextButton(
                  onPressed: _busy ? null : () => _replay(false),
                  child: const Text('再実行の承認待ち要求を作成')),
              TextButton(
                  onPressed: _busy ? null : () => _replay(true),
                  child: const Text('分岐の承認待ち要求を作成')),
              TextButton(
                  onPressed: _busy ? null : () => setState(_clearSelection),
                  child: const Text('選択を解除')),
            ]),
          ],
          if (page != null && _created != null)
            SelectableText(
                '新要求は承認待ちです。別途owner承認が必要です。\n要求ID: ${_created!['要求ID']}\n要求hash: ${_created!['要求hash']}\n新Session: ${_created!['対話セッションID']}\nRuntime: ${_created!['実行系ID']}\n作成種別: ${_created!['種別']}\n参照元監査: ${_created!['参照監査ID']}\n参照元監査hash: ${_created!['参照event_hash']}'),
          if (page != null)
            Text('Runtime: ${page.grant.runtime} ／ ${page.entries.length}件の要求'),
          if (page != null && page.entries.isEmpty) const Text('該当する履歴はありません。'),
          Expanded(
              child: ListView.builder(
                  itemCount: page?.entries.length ?? 0,
                  itemBuilder: (context, index) {
                    final e = page!.entries[index];
                    final r = e.record;
                    return ExpansionTile(
                        key: ValueKey(e.auditId),
                        initiallyExpanded: _expanded.contains(e.auditId),
                        onExpansionChanged: (open) {
                          if (open) {
                            _expanded.add(e.auditId);
                          } else {
                            _expanded.remove(e.auditId);
                          }
                        },
                        title: Text('${e.state} ／ ${r.fields['要求ID']}'),
                        subtitle: Text('開始 ${r.time('開始時刻')}'),
                        children: [
                          if (e.receipt != null) ...[
                            TextButton(
                                onPressed: _busy
                                    ? null
                                    : () => showDialog<void>(
                                        context: context,
                                        builder: (_) => HistoryContentDialog(
                                            client: _client!, entry: e)),
                                child: const Text('現在承認で保存内容を開く')),
                            SelectableText(
                                '保存監査: ${e.receipt!.auditId}\n保存監査hash: ${e.receipt!.eventHash}\n暗号文hash: ${e.receipt!.cipherHash}\n内容閲覧には別途ownerの現在承認が必要です。'),
                          ] else
                            const Text('内容の保存記録なし'),
                          TextButton(
                              onPressed: _busy
                                  ? null
                                  : () => setState(() {
                                        _clearSelection();
                                        _selected = e;
                                      }),
                              child: const Text('再実行・分岐の入力へ')),
                          SelectableText(
                              'Session: ${r.fields['対話セッションID']}\n入力概要: ${e.inputSummary == null ? '未記録（旧形式）' : 'hash_only／${e.inputSummary!.hash}'}\n入力本文は返しません。hashは候補照合に使えますが、本文の正しさ・現在権限は示しません。\n終了: ${r.time('終了時刻')}\n失敗分類: ${e.failure ?? 'なし'}\n履歴監査: ${e.auditId}\n作成監査: ${r.audit('作成監査ID')}\n開始監査: ${r.audit('開始監査ID')}\n終了監査: ${r.audit('終了監査ID')}\n作成操作: 対話送信\n要求hash: ${e.context.requestHash}\n過去承認: ${e.context.approvalId ?? '未記録'}\n承認時の表示範囲: ${e.context.scope ?? '未記録'}\n承認能力: ${e.context.approvalId == null ? '未記録' : '対話送信'}\n復旧対応: ${e.context.approvalId == null ? '未記録' : '接続再確認'}\n過去の承認記録です。現在の権限・能力使用・復旧実施を示しません。\n結果証跡: ${e.evidence == null ? '未記録' : 'Adapter申告のhash記録（実使用証明ではありません）'}\n応答hash: ${e.evidence?.responseHash ?? '未記録'}\n使用能力（Adapter申告hash）: ${e.evidence == null ? '未記録' : e.evidence!.capabilityHash ?? '非保存'}\n経路（Adapter申告hash）: ${e.evidence == null ? '未記録' : e.evidence!.routeHash ?? '非保存'}\n追跡参照hash: ${e.evidence == null ? '未記録' : e.evidence!.traceHash ?? '非保存'}')
                        ]);
                  })),
        ]));
  }
}

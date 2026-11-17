import 'dart:async';
import 'package:flutter/material.dart';
import 'package:gui_shell_ui/gui_shell_ui.dart';
import '../services/broker_client.dart';
import 'agent_task_history_branch_dialog.dart';
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
  AgentTaskHistoryPage? _taskPage;
  HistoryGrant? _grant;
  bool _showTaskHistory = false;
  Timer? _expiryTimer, _grantCheckTimer, _replayRefreshTimer;
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
  void didUpdateWidget(covariant HistoryScreen oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (identical(oldWidget.client?.transport, widget.client?.transport)) {
      return;
    }

    _generation++;
    _expiryTimer?.cancel();
    _grantCheckTimer?.cancel();
    _replayRefreshTimer?.cancel();
    _client = widget.client;
    _page = null;
    _taskPage = null;
    _grant = null;
    _busy = false;
    _showTaskHistory = false;
    _requestId = null;
    _sessionId = null;
    _state = null;
    _requestFilter.clear();
    _sessionFilter.clear();
    _clearSelection();
    _message = '履歴接続先が変わりました。現在の接続先を確認中です。';

    final transport = widget.client?.transport;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted &&
          _active &&
          identical(widget.client?.transport, transport)) {
        _load();
      }
    });
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    _active = state == AppLifecycleState.resumed;
    _generation++;
    _expiryTimer?.cancel();
    _grantCheckTimer?.cancel();
    _replayRefreshTimer?.cancel();
    setState(() {
      _page = null;
      _taskPage = null;
      _grant = null;
      _busy = false;
      _clearSelection();
      _message = '画面復帰後は更新してください。';
    });
  }

  @override
  void dispose() {
    _generation++;
    _expiryTimer?.cancel();
    _grantCheckTimer?.cancel();
    _replayRefreshTimer?.cancel();
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
    _grantCheckTimer?.cancel();
    _replayRefreshTimer?.cancel();
    final generation = ++_generation;
    final previousGrant = _grant;
    final restoreInputFocus = _inputFocus.hasFocus;
    setState(() {
      _page = null;
      _taskPage = null;
      _busy = true;
      _message = '現在承認と履歴を確認中';
    });
    try {
      HistoryClient? client = _client ?? widget.client;
      if (client == null) {
        final transport = await BrokerClient.connect();
        if (!mounted || !_active || generation != _generation) return;
        client = HistoryClient(transport);
      }
      if (!mounted || !_active || generation != _generation) return;
      _client = client;
      final activeClient = client;
      final grant = await activeClient.status();
      if (grant == null) {
        throw const BrokerClientException('ownerの履歴閲覧承認がありません。');
      }
      if (_showTaskHistory) {
        final taskPage = await activeClient.taskPage(grant, after: after);
        if (!mounted || !_active || generation != _generation) return;
        setState(() {
          if (previousGrant != null && !previousGrant.same(taskPage.grant)) {
            _clearSelection();
          }
          _taskPage = taskPage;
          _grant = taskPage.grant;
          _message =
              'Agent Taskの過去状態です。これは取得時点の情報です。「先頭から更新」で再取得します。本文・現在の権限・実行状態を示しません。';
        });
        _scheduleExpiry(taskPage.grant);
      } else {
        final page = await activeClient.page(grant,
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
          _grant = page.grant;
          _message = '取得時点の一覧です。「先頭から更新」で再取得します。現在の稼働・実行許可を示しません。';
        });
        _scheduleExpiry(page.grant);
      }
      if (restoreInputFocus && !_showTaskHistory && _selected != null) {
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
    } catch (_) {
      if (mounted && generation == _generation) {
        setState(() {
          _page = null;
          _taskPage = null;
          _grant = null;
          _clearSelection();
          _message = '現在の履歴閲覧承認または接続を確認できません。';
        });
      }
    } finally {
      if (mounted && generation == _generation) setState(() => _busy = false);
      final grant = _grant;
      if (mounted &&
          generation == _generation &&
          grant != null &&
          (_page != null || _taskPage != null)) {
        _scheduleGrantCheck(generation, grant);
      }
    }
  }

  // Grant revocation still clears the view promptly. Keep the larger history
  // page point-in-time and reread it only after an explicit user action.
  void _scheduleGrantCheck(int generation, HistoryGrant grant) {
    _grantCheckTimer?.cancel();
    if (!mounted ||
        !_active ||
        generation != _generation ||
        _grant == null ||
        !_grant!.same(grant)) {
      return;
    }
    _grantCheckTimer = Timer(const Duration(seconds: 2), () {
      _grantCheckTimer = null;
      unawaited(_checkGrant(generation, grant));
    });
  }

  Future<void> _checkGrant(int generation, HistoryGrant grant) async {
    final client = _client;
    if (!mounted ||
        !_active ||
        _busy ||
        generation != _generation ||
        client == null) {
      return;
    }
    try {
      final current = await client.status();
      if (!mounted || !_active || generation != _generation) return;
      if (current == null || !grant.same(current) || !current.current) {
        _clearHistoryForGrantFailure(
          '現在の履歴閲覧承認を確認できないため表示を消去しました。',
        );
        return;
      }
      _scheduleGrantCheck(generation, current);
    } catch (_) {
      if (mounted && _active && generation == _generation) {
        _clearHistoryForGrantFailure(
          '現在の履歴閲覧承認を確認できないため表示を消去しました。',
        );
      }
    }
  }

  void _clearHistoryForGrantFailure(String message) {
    _generation++;
    _expiryTimer?.cancel();
    _expiryTimer = null;
    _grantCheckTimer?.cancel();
    _grantCheckTimer = null;
    _replayRefreshTimer?.cancel();
    _replayRefreshTimer = null;
    setState(() {
      _page = null;
      _taskPage = null;
      _grant = null;
      _busy = false;
      _clearSelection();
      _message = message;
    });
  }

  void _scheduleExpiry(HistoryGrant grant) {
    _expiryTimer?.cancel();
    if (!mounted || _grant == null || !_grant!.same(grant)) return;
    _expiryTimer = Timer(grant.untilExpiry, () {
      if (!mounted || _grant == null || !_grant!.same(grant)) return;
      if (grant.current) {
        _scheduleExpiry(grant);
        return;
      }
      _generation++;
      _grantCheckTimer?.cancel();
      _grantCheckTimer = null;
      _replayRefreshTimer?.cancel();
      _replayRefreshTimer = null;
      setState(() {
        _page = null;
        _taskPage = null;
        _grant = null;
        _busy = false;
        _clearSelection();
        _message = '閲覧期限を過ぎました。';
      });
    });
  }

  void _selectHistory(bool taskHistory) {
    if (_busy || _showTaskHistory == taskHistory) return;
    _grantCheckTimer?.cancel();
    _replayRefreshTimer?.cancel();
    _clearSelection();
    setState(() {
      _showTaskHistory = taskHistory;
      _page = null;
      _taskPage = null;
    });
    _load();
  }

  Future<void> _replay(bool branch) async {
    final page = _page;
    final selected = _selected;
    if (_busy || page == null || selected == null) return;
    _grantCheckTimer?.cancel();
    _replayRefreshTimer?.cancel();
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
        _replayRefreshTimer = Timer(const Duration(seconds: 2), () {
          _replayRefreshTimer = null;
          if (mounted && _active && generation == _generation) _load();
        });
      }
    }
  }

  Future<void> _branchAgentTask(AgentTaskHistoryEntry entry) async {
    final client = _client;
    final grant = _grant;
    if (_busy ||
        !_active ||
        client == null ||
        grant == null ||
        !grant.current ||
        entry.runtimeId != grant.runtime ||
        !const {'completed', 'failed', 'cancelled', 'suspended'}
            .contains(entry.status)) {
      return;
    }
    await showDialog<void>(
      context: context,
      builder: (_) => AgentTaskHistoryBranchDialog(
        client: client,
        grant: grant,
        source: entry,
      ),
    );
    if (mounted && _active) _load(after: 0);
  }

  @override
  Widget build(BuildContext context) {
    final page = _page;
    final taskPage = _taskPage;
    final hasMore =
        _showTaskHistory ? taskPage?.more == true : page?.more == true;
    return Padding(
        padding: const EdgeInsets.all(16),
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text('履歴', style: Theme.of(context).textTheme.headlineSmall),
          Text(_message),
          SegmentedButton<bool>(
              segments: const [
                ButtonSegment(value: false, label: Text('実行履歴')),
                ButtonSegment(value: true, label: Text('Agent Task履歴')),
              ],
              selected: {
                _showTaskHistory
              },
              onSelectionChanged: _busy
                  ? null
                  : (selection) => _selectHistory(selection.single)),
          if (!_showTaskHistory)
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
                      decoration: const InputDecoration(
                          labelText: 'Session ID（完全一致）'))),
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
            if (!_showTaskHistory)
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
                onPressed: hasMore && !_busy
                    ? () => _load(
                        after: _showTaskHistory ? taskPage!.next : page!.next)
                    : null,
                child: const Text('次のページ')),
          ]),
          if (_busy) const LinearProgressIndicator(),
          if (!_showTaskHistory && page != null && _selected != null) ...[
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
          if (!_showTaskHistory && page != null && _created != null)
            SelectableText(
                '新要求は承認待ちです。別途owner承認が必要です。\n要求ID: ${_created!['要求ID']}\n要求hash: ${_created!['要求hash']}\n新Session: ${_created!['対話セッションID']}\nRuntime: ${_created!['実行系ID']}\n作成種別: ${_created!['種別']}\n参照元監査: ${_created!['参照監査ID']}\n参照元監査hash: ${_created!['参照event_hash']}'),
          if (!_showTaskHistory && page != null)
            Text('Runtime: ${page.grant.runtime} ／ ${page.entries.length}件の要求'),
          if (!_showTaskHistory && page != null && page.entries.isEmpty)
            const Text('該当する履歴はありません。'),
          if (_showTaskHistory && taskPage != null)
            Text(
                'Runtime: ${taskPage.grant.runtime} ／ ${taskPage.entries.length}件のAgent Task'),
          if (_showTaskHistory && taskPage != null)
            const Text(
                '構造化履歴記録を持つTaskのみ表示します。導入前の記録はAuditに保持されますが、この一覧へ推測再構成しません。'),
          if (_showTaskHistory && taskPage != null && taskPage.entries.isEmpty)
            const Text('該当するAgent Task履歴はありません。'),
          Expanded(
              child: _showTaskHistory
                  ? ListView.builder(
                      itemCount: taskPage?.entries.length ?? 0,
                      itemBuilder: (context, index) {
                        final entry = taskPage!.entries[index];
                        final created = DateTime.fromMillisecondsSinceEpoch(
                                entry.createdAt * 1000,
                                isUtc: true)
                            .toLocal();
                        final updated = DateTime.fromMillisecondsSinceEpoch(
                                entry.updatedAt * 1000,
                                isUtc: true)
                            .toLocal();
                        return ExpansionTile(
                          key: ValueKey('agent-task-${entry.taskId}'),
                          title: Text('${entry.status} ／ ${entry.taskId}'),
                          subtitle: Text(
                              'Workspace: ${entry.workspaceId} ／ 更新: $updated'),
                          children: [
                            SelectableText(
                                'Session: ${entry.sessionId}\nRuntime: ${entry.runtimeId}\n指示hash: ${entry.instructionHash}\n結果hash: ${entry.resultHash ?? 'なし'}\n失敗分類: ${entry.failureClass ?? 'なし'}\n作成: $created\n開始Audit: ${entry.startAuditEventId}\n最新Audit: ${entry.auditId}\nこれはAudit由来の過去状態です。本文・Permission・Approval・Credential・Authorityは復元しません。'),
                            if (const {
                              'completed',
                              'failed',
                              'cancelled',
                              'suspended'
                            }.contains(entry.status))
                              TextButton(
                                key: ValueKey(
                                    'agent-task-branch-${entry.taskId}'),
                                onPressed: _busy
                                    ? null
                                    : () => _branchAgentTask(entry),
                                child: const Text('新しいAgent Taskへ分岐'),
                              ),
                          ],
                        );
                      })
                  : ListView.builder(
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
                                            builder: (_) =>
                                                HistoryContentDialog(
                                                    client: _client!,
                                                    entry: e)),
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

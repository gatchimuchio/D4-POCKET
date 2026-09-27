import 'dart:async';
import 'package:flutter/material.dart';
import 'regression_case_owner_client.dart';
import 'runtime_dialogue_client.dart';

/// 対話の表示と入力だけを所有する。実行の採否はbrokerから取得する。
class RuntimeDialogueScreen extends StatefulWidget {
  const RuntimeDialogueScreen(
      {super.key,
      required this.connect,
      this.client,
      this.connectOwnerRegistration,
      this.readOnly = false,
      this.active = true,
      this.workspaceSelectionSupported = true});
  final Future<RuntimeDialogueClient> Function() connect;
  final RuntimeDialogueClient? client;
  final Future<RegressionCaseOwnerClient> Function()? connectOwnerRegistration;
  final bool readOnly;
  final bool active;
  final bool workspaceSelectionSupported;
  @override
  State<RuntimeDialogueScreen> createState() => _RuntimeDialogueScreenState();
}

class _Conversation {
  String? runtime;
  String? workspaceId;
  String? session;
  String? request;
  String? requestHash;
  String state = '未開始';
  String? error;
  DialogueResult? result;
  DialogueExecutionRecord? record;
  RegressionCaseRegistrationReceipt? registrationReceipt;
  bool pending = false;
  bool busy = false;
  bool polling = false;
}

class _RuntimeDialogueScreenState extends State<RuntimeDialogueScreen> {
  RuntimeDialogueClient? _client;
  final _input = TextEditingController();
  final _left = _Conversation();
  final _right = _Conversation();
  List<String> _runtimes = [];
  Map<String, List<String>> _workspaceIdsByRuntime = const {};
  String? _workspaceLoadWarning;
  String? _connectionError;
  bool _connecting = false;
  bool _compare = false;
  bool _sending = false;
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    unawaited(_connect());
    _timer =
        Timer.periodic(const Duration(seconds: 1), (_) => unawaited(_poll()));
  }

  Future<void> _connect() async {
    if (_connecting || !widget.active) return;
    setState(() {
      _connecting = true;
      _connectionError = null;
    });
    try {
      final client = widget.client ?? await widget.connect();
      final runtimes = await client.runtimes();
      Map<String, List<String>> workspaceIds = const {};
      String? workspaceWarning;
      if (widget.workspaceSelectionSupported) {
        try {
          workspaceIds = await client.workspaceIdsByRuntime();
        } catch (_) {
          workspaceWarning = 'Workspace一覧を検証できません。Agent開始はBroker側で拒否されます。';
        }
      } else {
        workspaceWarning = 'この接続面はWorkspace選択に未対応です。Agent対話開始はBrokerが拒否します。';
      }
      if (!mounted) return;
      setState(() {
        _client = client;
        _runtimes = runtimes;
        _workspaceIdsByRuntime = workspaceIds;
        _workspaceLoadWarning = workspaceWarning;
        _left.runtime ??= runtimes.firstOrNull;
        _right.runtime ??= runtimes.length > 1 ? runtimes[1] : null;
      });
    } catch (_) {
      if (mounted) {
        setState(() => _connectionError = 'brokerに接続できません。起動と通常接続資格を確認してください。');
      }
    } finally {
      if (mounted) setState(() => _connecting = false);
    }
  }

  Future<void> _newSession(_Conversation side) async {
    if (!widget.active || widget.readOnly) return;
    final client = _client;
    final runtime = side.runtime;
    if (client == null || runtime == null || side.busy || side.pending) return;
    setState(() {
      side.busy = true;
      side.error = null;
    });
    try {
      if (side.session != null) await client.close(side.session!);
      if (!mounted) return;
      setState(() {
        side.session = null;
        side.request = null;
        side.requestHash = null;
        side.result = null;
        side.record = null;
        side.registrationReceipt = null;
        side.state = '未開始';
      });
      if (!widget.active) return;
      final session =
          await client.start(runtime, workspaceId: side.workspaceId);
      if (!mounted) {
        await client.close(session);
        return;
      }
      setState(() {
        side.session = session;
        side.request = null;
        side.requestHash = null;
        side.result = null;
        side.record = null;
        side.registrationReceipt = null;
        side.state = '入力待ち';
      });
    } catch (_) {
      if (mounted) {
        setState(() => side.error = 'セッションを開始できません。取消直後は処理の終了を待って再試行してください。');
      }
    } finally {
      if (mounted) setState(() => side.busy = false);
    }
  }

  Future<void> _sendSide(_Conversation side, String input) async {
    final client = _client!;
    if (side.session == null) await _newSession(side);
    if (!mounted || !widget.active || widget.readOnly || side.session == null) {
      return;
    }
    setState(() {
      side.busy = true;
      side.result = null;
      side.record = null;
      side.registrationReceipt = null;
      side.error = null;
    });
    try {
      final request = await client.sendReference(side.session!, input);
      if (!mounted) {
        await client.cancel(request.requestId);
        return;
      }
      setState(() {
        side.request = request.requestId;
        side.requestHash = request.requestHash;
        side.state = '承認待ち';
        side.pending = true;
      });
    } catch (_) {
      if (mounted) {
        setState(() {
          side.error = '送信を確認できません。自動再送は行いません。新規セッションで状態を確認してください。';
          side.state = '送信失敗';
        });
      }
    } finally {
      if (mounted) setState(() => side.busy = false);
    }
  }

  Future<void> _send() async {
    if (widget.readOnly || !widget.active) return;
    final input = _input.text;
    if (_sending ||
        _client == null ||
        input.trim().isEmpty ||
        input.runes.length > 4096) {
      return;
    }
    if (_left.runtime == null ||
        (_compare &&
            (_right.runtime == null || _left.runtime == _right.runtime))) {
      setState(() => _connectionError = '比較には異なる二つの実行系を選択してください。');
      return;
    }
    setState(() => _sending = true);
    try {
      // 同一の不変入力を渡し、要求・session・応答は左右で独立させる。
      await Future.wait(
          [_sendSide(_left, input), if (_compare) _sendSide(_right, input)]);
    } finally {
      if (mounted) setState(() => _sending = false);
    }
  }

  Future<void> _poll() async {
    await Future.wait([_pollSide(_left), _pollSide(_right)]);
  }

  Future<void> _pollSide(_Conversation side) async {
    final client = _client;
    if (client == null ||
        !mounted ||
        !widget.active ||
        side.polling ||
        !side.pending ||
        side.busy ||
        side.request == null) {
      return;
    }
    final request = side.request!;
    side.polling = true;
    try {
      final progress = await client.poll(request, side.runtime!, side.session!);
      if (!mounted || side.request != request || !side.pending) return;
      setState(() {
        side.state = progress.state;
        side.result = progress.result;
        side.record = progress.record;
        side.error = null;
        side.pending = progress.result == null;
      });
    } catch (_) {
      if (mounted && side.request == request && side.pending) {
        setState(() {
          side.error = '応答を取得できません。接続・監査を確認してください。本文の表示は保留しています。';
          side.result = null;
          side.record = null;
        });
      }
    } finally {
      side.polling = false;
    }
  }

  Future<void> _cancel(_Conversation side) async {
    if (!widget.active || widget.readOnly) return;
    if (!side.pending || side.request == null || side.busy) return;
    setState(() => side.busy = true);
    try {
      await _client!.cancel(side.request!);
      if (mounted) {
        setState(() {
          side.pending = false;
          side.result = null;
          side.record = null;
          side.state = '中止';
          side.error = null;
        });
      }
    } catch (_) {
      if (mounted) {
        setState(() => side.error = '中止を確認できません。実行系とbrokerの状態を確認してください。');
      }
    } finally {
      if (mounted) setState(() => side.busy = false);
    }
  }

  Future<void> _closeSession(_Conversation side) async {
    if (!widget.active || widget.readOnly) return;
    if (side.busy || side.pending || side.session == null) return;
    setState(() => side.busy = true);
    try {
      await _client!.close(side.session!);
      if (!mounted) return;
      setState(() {
        side.session = null;
        side.request = null;
        side.requestHash = null;
        side.result = null;
        side.record = null;
        side.registrationReceipt = null;
        side.state = '未開始';
        side.error = null;
      });
    } catch (_) {
      if (mounted) {
        setState(() => side.error = 'セッションを終了できません。処理の終了を待ってください。');
      }
    } finally {
      if (mounted) {
        setState(() => side.busy = false);
      }
    }
  }

  bool _canRegister(_Conversation side) {
    final result = side.result;
    final record = side.record;
    return widget.connectOwnerRegistration != null &&
        !widget.readOnly &&
        widget.active &&
        side.request != null &&
        side.requestHash != null &&
        result != null &&
        result.text('表示範囲') == 'full' &&
        const {'成功', '保留'}.contains(result.text('状態')) &&
        record != null &&
        record.audit('終了監査ID') != '未記録';
  }

  Future<void> _registerCase(_Conversation side) async {
    if (!_canRegister(side) || side.busy || side.pending) return;
    final requestId = side.request!;
    final requestHash = side.requestHash!;
    final expectedStatus = side.result!.text('状態');
    final receipt = await showDialog<RegressionCaseRegistrationReceipt>(
      context: context,
      barrierDismissible: false,
      builder: (context) => _RegressionCaseRegistrationDialog(
        requestId: requestId,
        requestHash: requestHash,
        expectedStatus: expectedStatus,
        connectOwner: widget.connectOwnerRegistration!,
      ),
    );
    if (receipt != null && mounted) {
      setState(() => side.registrationReceipt = receipt);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('回帰Caseを保存しました: ${receipt.caseId}')),
      );
    }
  }

  @override
  void didUpdateWidget(covariant RuntimeDialogueScreen oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.active && !oldWidget.active) {
      if (_client == null) {
        unawaited(_connect());
      } else {
        unawaited(_poll());
      }
    }
  }

  @override
  void dispose() {
    _timer?.cancel();
    _input.dispose();
    final client = _client;
    if (client != null) {
      for (final side in [_left, _right]) {
        if (side.pending && side.request != null) {
          // 終了時の通信失敗を成功に見せない。次起動時は新sessionで確認する。
          unawaited(client.cancel(side.request!).catchError((Object _) {}));
        } else if (side.session != null) {
          unawaited(client.close(side.session!).catchError((Object _) {}));
        }
      }
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final active = !widget.active ||
        widget.readOnly ||
        _sending ||
        _left.pending ||
        _right.pending ||
        _left.busy ||
        _right.busy;
    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
        Text('実行系との対話', style: Theme.of(context).textTheme.headlineSmall),
        if (widget.readOnly) const Text('デモ表示では送信・セッション変更はできません。'),
        if (!widget.active) const Text('接続確認または画面復帰まで通信を停止しています。'),
        const SizedBox(height: 8),
        const Text('送信後はownerの承認を待ちます。中止は応答の採用を止めますが、実行系の処理停止や巻戻しを保証しません。'),
        if (widget.workspaceSelectionSupported)
          const Text(
              'Agent Runtimeでは同じRuntimeに登録されたWorkspace IDを選択してください。IDの結合は実行directory・権限・書込み隔離の証明ではなく、二実行系表示も実Agent隔離比較ではありません。'),
        if (_workspaceLoadWarning != null)
          Text(_workspaceLoadWarning!,
              style: TextStyle(color: Theme.of(context).colorScheme.error)),
        if (_connectionError != null)
          Padding(
              padding: const EdgeInsets.symmetric(vertical: 8),
              child: Text(_connectionError!,
                  style:
                      TextStyle(color: Theme.of(context).colorScheme.error))),
        if (_client == null)
          Align(
              alignment: Alignment.centerLeft,
              child: FilledButton.tonal(
                  onPressed: _connecting || !widget.active ? null : _connect,
                  child: Text(_connecting ? '接続中' : '接続を再試行'))),
        if (_client != null && _runtimes.isEmpty)
          const Text('登録された実行系がありません。起動時の実行系登録を確認してください。'),
        SwitchListTile(
            title: const Text('二実行系を比較'),
            subtitle: const Text('同じ入力を独立した要求として送信します'),
            value: _compare,
            onChanged: active ? null : (v) => setState(() => _compare = v)),
        LayoutBuilder(builder: (context, constraints) {
          final left = _pane(_left, _compare ? '左の実行系' : '実行系', active);
          final right = _pane(_right, '右の実行系', active);
          if (!_compare) return left;
          if (constraints.maxWidth < 760) {
            return Column(children: [left, const SizedBox(height: 12), right]);
          }
          return Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Expanded(child: left),
            const SizedBox(width: 16),
            Expanded(child: right)
          ]);
        }),
        const SizedBox(height: 16),
        TextField(
            controller: _input,
            enabled: !active,
            minLines: 2,
            maxLines: 8,
            maxLength: 4096,
            decoration: const InputDecoration(
                labelText: '入力', border: OutlineInputBorder())),
        Align(
            alignment: Alignment.centerLeft,
            child: FilledButton.icon(
                key: const ValueKey('dialogue-send'),
                onPressed: active || _client == null ? null : _send,
                icon: const Icon(Icons.send),
                label: Text(_compare ? '同じ入力を両側へ送信' : '送信'))),
      ]),
    );
  }

  Widget _pane(_Conversation side, String label, bool active) {
    final result = side.result;
    return Card(
        child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                DropdownButtonFormField<String>(
                    initialValue: side.runtime,
                    isExpanded: true,
                    decoration: InputDecoration(labelText: label),
                    items: _runtimes
                        .map((v) => DropdownMenuItem(value: v, child: Text(v)))
                        .toList(),
                    onChanged: active || side.session != null
                        ? null
                        : (v) => setState(() {
                              side.runtime = v;
                              side.workspaceId = null;
                            })),
                if (widget.workspaceSelectionSupported &&
                    side.runtime != null) ...[
                  if ((_workspaceIdsByRuntime[side.runtime] ?? []).isEmpty)
                    const Text(
                        'このRuntimeに登録されたWorkspaceはありません。Agent Adapterは未指定で開始できません。')
                  else
                    DropdownButtonFormField<String>(
                      key: ValueKey('dialogue-workspace-${side.runtime}'),
                      initialValue: (_workspaceIdsByRuntime[side.runtime] ?? [])
                              .contains(side.workspaceId)
                          ? side.workspaceId
                          : '',
                      isExpanded: true,
                      decoration: const InputDecoration(
                          labelText: '対話Sessionへ結合するWorkspace ID'),
                      items: [
                        const DropdownMenuItem(
                            value: '', child: Text('指定しない（Agentでは不可）')),
                        for (final id
                            in _workspaceIdsByRuntime[side.runtime] ?? [])
                          DropdownMenuItem(value: id, child: Text(id)),
                      ],
                      onChanged: active || side.session != null
                          ? null
                          : (value) => setState(() => side.workspaceId =
                              value == null || value.isEmpty ? null : value),
                    ),
                ],
                Wrap(spacing: 8, children: [
                  OutlinedButton(
                      onPressed: active || side.runtime == null
                          ? null
                          : () => _newSession(side),
                      child: const Text('新規セッション')),
                  if (side.session != null)
                    TextButton(
                        onPressed: active ? null : () => _closeSession(side),
                        child: const Text('セッション終了')),
                  OutlinedButton(
                      onPressed: widget.active &&
                              !widget.readOnly &&
                              side.pending &&
                              !side.busy
                          ? () => _cancel(side)
                          : null,
                      child: const Text('中止')),
                ]),
                Text('状態: ${side.state}'),
                if (side.session != null)
                  SelectableText('セッション: ${side.session}'),
                if (side.request != null) SelectableText('要求: ${side.request}'),
                if (side.requestHash != null)
                  SelectableText('要求hash: ${side.requestHash}'),
                if (side.state == '承認待ち') const Text('ownerの承認操作を待っています。'),
                if (side.request != null && side.record == null)
                  const Text('実行記録は未取得です。'),
                if (side.record != null)
                  ExpansionTile(
                    key: ValueKey('execution-record-${side.request}'),
                    title: const Text('実行記録'),
                    children: [
                      const Text('Brokerの観測時刻（UTC）。外部処理の停止証拠ではありません。'),
                      for (final stage in ['作成', '開始', '終了']) ...[
                        SelectableText(
                            '$stage: ${side.record!.time('$stage時刻')}'),
                        SelectableText(
                            '$stage監査: ${side.record!.audit('$stage監査ID')}'),
                      ],
                    ],
                  ),
                if (side.error != null)
                  Text(side.error!,
                      style: TextStyle(
                          color: Theme.of(context).colorScheme.error)),
                if (result != null) ...[
                  const Divider(),
                  Text('応答状態: ${result.text('状態')}'),
                  Text('表示範囲: ${result.text('表示範囲')}'),
                  if (result.text('本文').isNotEmpty)
                    SelectableText(result.text('本文')),
                  if (result.text('失敗分類').isNotEmpty)
                    Text(
                        '失敗: ${result.text('失敗分類')} / 復旧: ${result.text('復旧')}'),
                  if (result.list('参照').isNotEmpty)
                    SelectableText('参照\n${result.list('参照').join('\n')}'),
                  if (result.list('能力').isNotEmpty)
                    Text('能力: ${result.list('能力').join('、')}'),
                  for (final key in ['経路', '追跡ID', '追跡hash', '応答hash'])
                    if (result.text(key).isNotEmpty)
                      SelectableText('$key: ${result.text(key)}'),
                  if (_canRegister(side))
                    Align(
                      alignment: Alignment.centerLeft,
                      child: OutlinedButton.icon(
                        key: ValueKey('dialogue-register-case-${side.request}'),
                        onPressed: active || side.busy
                            ? null
                            : () => _registerCase(side),
                        icon: const Icon(Icons.bookmark_add_outlined),
                        label: const Text('この完了結果から回帰Caseを登録'),
                      ),
                    ),
                ],
                if (side.registrationReceipt != null)
                  SelectableText(
                    '登録済み回帰事例: ${side.registrationReceipt!.caseId}\n'
                    '定義ハッシュ: ${side.registrationReceipt!.definitionHash}\n'
                    '監査ID: ${side.registrationReceipt!.auditId}',
                  ),
              ],
            )));
  }
}

class _RegressionCaseRegistrationDialog extends StatefulWidget {
  const _RegressionCaseRegistrationDialog({
    required this.requestId,
    required this.requestHash,
    required this.expectedStatus,
    required this.connectOwner,
  });

  final String requestId;
  final String requestHash;
  final String expectedStatus;
  final Future<RegressionCaseOwnerClient> Function() connectOwner;

  @override
  State<_RegressionCaseRegistrationDialog> createState() =>
      _RegressionCaseRegistrationDialogState();
}

class _RegressionCaseRegistrationDialogState
    extends State<_RegressionCaseRegistrationDialog> {
  final _formKey = GlobalKey<FormState>();
  final _displayName = TextEditingController();
  final _redactedInput = TextEditingController();
  final _requiredConditions = TextEditingController();
  final _forbiddenConditions = TextEditingController();
  final _requiredReferences = TextEditingController();
  final _expectedRoute = TextEditingController();
  String? _error;
  bool _busy = false;

  List<String> _lines(TextEditingController controller) => controller.text
      .split('\n')
      .map((value) => value.trim())
      .where((value) => value.isNotEmpty)
      .toList(growable: false);

  String? _listError(String? value, int maximumItems, int maximumLength) {
    final entries = (value ?? '')
        .split('\n')
        .map((entry) => entry.trim())
        .where((entry) => entry.isNotEmpty)
        .toList(growable: false);
    if (entries.length > maximumItems ||
        entries.any((entry) => entry.runes.length > maximumLength)) {
      return '1行1件で、件数・文字数の上限内にしてください';
    }
    return null;
  }

  Future<void> _submit() async {
    if (_busy || !_formKey.currentState!.validate()) return;
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final owner = await widget.connectOwner();
      final receipt = await owner.register(
        requestId: widget.requestId,
        requestHash: widget.requestHash,
        displayName: _displayName.text,
        redactedInput: _redactedInput.text,
        requiredConditions: _lines(_requiredConditions),
        forbiddenConditions: _lines(_forbiddenConditions),
        expectedStatus: widget.expectedStatus,
        requiredReferences: _lines(_requiredReferences),
        expectedRoute: _expectedRoute.text,
      );
      if (mounted) Navigator.of(context).pop(receipt);
    } catch (_) {
      if (mounted) {
        setState(() {
          _busy = false;
          _error = '登録を確認できません。対象結果、Windows確認、Broker監査を確認してください。';
        });
      }
    }
  }

  @override
  void dispose() {
    _displayName.dispose();
    _redactedInput.dispose();
    _requiredConditions.dispose();
    _forbiddenConditions.dispose();
    _requiredReferences.dispose();
    _expectedRoute.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
        title: const Text('回帰Caseのowner登録'),
        content: SizedBox(
          width: 560,
          child: Form(
            key: _formKey,
            child: SingleChildScrollView(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text('対象要求ID: ${widget.requestId}'),
                  SelectableText('対象要求hash: ${widget.requestHash}'),
                  Text('Brokerで完了確認済みの結果状態: ${widget.expectedStatus}'),
                  const SizedBox(height: 12),
                  const Text(
                    '元の対話本文は自動コピーしません。下記には秘密を除いた再現用定義だけを記入してください。'
                    '既知の秘密候補検査は、秘密が含まれないことを保証しません。登録内容はprivate保管されます。',
                  ),
                  const SizedBox(height: 12),
                  TextFormField(
                    key: const ValueKey('regression-registration-name'),
                    controller: _displayName,
                    maxLength: 128,
                    decoration: const InputDecoration(
                      labelText: '公開表示名',
                      helperText: '通常一覧に表示されます',
                    ),
                    validator: (value) => value == null || value.trim().isEmpty
                        ? '入力してください'
                        : null,
                  ),
                  TextFormField(
                    key: const ValueKey('regression-registration-input'),
                    controller: _redactedInput,
                    minLines: 2,
                    maxLines: 5,
                    maxLength: 4096,
                    decoration: const InputDecoration(
                      labelText: '秘密を除いた再現入力',
                      helperText: '対話時の入力を貼り付けず、必要な内容を自分で書き直してください',
                    ),
                    validator: (value) => value == null || value.trim().isEmpty
                        ? 'サニタイズ済み入力を記入してください'
                        : null,
                  ),
                  TextFormField(
                    key: const ValueKey('regression-registration-required'),
                    controller: _requiredConditions,
                    minLines: 1,
                    maxLines: 4,
                    decoration: const InputDecoration(
                      labelText: '必要条件（1行1件、任意）',
                    ),
                    validator: (value) => _listError(value, 16, 512),
                  ),
                  TextFormField(
                    key: const ValueKey('regression-registration-forbidden'),
                    controller: _forbiddenConditions,
                    minLines: 1,
                    maxLines: 4,
                    decoration: const InputDecoration(
                      labelText: '禁止条件（1行1件、任意）',
                    ),
                    validator: (value) => _listError(value, 16, 512),
                  ),
                  TextFormField(
                    key: const ValueKey('regression-registration-references'),
                    controller: _requiredReferences,
                    minLines: 1,
                    maxLines: 4,
                    decoration: const InputDecoration(
                      labelText: '必要参照（1行1件、任意）',
                    ),
                    validator: (value) => _listError(value, 64, 2048),
                  ),
                  TextFormField(
                    key: const ValueKey('regression-registration-route'),
                    controller: _expectedRoute,
                    maxLength: 256,
                    decoration: const InputDecoration(labelText: '期待経路'),
                    validator: (value) => value == null || value.trim().isEmpty
                        ? '入力してください'
                        : null,
                  ),
                  if (_error != null)
                    Padding(
                      padding: const EdgeInsets.only(top: 8),
                      child: Text(
                        _error!,
                        style: TextStyle(
                            color: Theme.of(context).colorScheme.error),
                      ),
                    ),
                ],
              ),
            ),
          ),
        ),
        actions: [
          TextButton(
            onPressed: _busy ? null : () => Navigator.of(context).pop(),
            child: const Text('キャンセル'),
          ),
          FilledButton(
            key: const ValueKey('regression-registration-submit'),
            onPressed: _busy ? null : _submit,
            child: Text(_busy ? 'native確認待ち' : '登録を依頼'),
          ),
        ],
      );
}

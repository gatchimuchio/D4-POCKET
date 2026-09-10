import 'dart:async';
import 'package:flutter/material.dart';
import '../services/runtime_dialogue_client.dart';

/// 対話の表示と入力だけを所有する。実行の採否はbrokerから取得する。
class RuntimeDialogueScreen extends StatefulWidget {
  const RuntimeDialogueScreen({super.key, this.client, this.readOnly = false});
  final RuntimeDialogueClient? client;
  final bool readOnly;
  @override
  State<RuntimeDialogueScreen> createState() => _RuntimeDialogueScreenState();
}

class _Conversation {
  String? runtime;
  String? session;
  String? request;
  String state = '未開始';
  String? error;
  DialogueResult? result;
  bool pending = false;
  bool busy = false;
}

class _RuntimeDialogueScreenState extends State<RuntimeDialogueScreen> {
  RuntimeDialogueClient? _client;
  final _input = TextEditingController();
  final _left = _Conversation();
  final _right = _Conversation();
  List<String> _runtimes = [];
  String? _connectionError;
  bool _connecting = false;
  bool _compare = false;
  bool _polling = false;
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
    if (_connecting) return;
    setState(() {
      _connecting = true;
      _connectionError = null;
    });
    try {
      final client = widget.client ?? await RuntimeDialogueClient.connect();
      final runtimes = await client.runtimes();
      if (!mounted) return;
      setState(() {
        _client = client;
        _runtimes = runtimes;
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
    final client = _client;
    final runtime = side.runtime;
    if (client == null || runtime == null || side.busy || side.pending) return;
    setState(() {
      side.busy = true;
      side.error = null;
    });
    try {
      if (side.session != null) await client.close(side.session!);
      side.session = null;
      final session = await client.start(runtime);
      if (!mounted) {
        await client.close(session);
        return;
      }
      setState(() {
        side.session = session;
        side.request = null;
        side.result = null;
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
    if (!mounted || side.session == null) return;
    setState(() {
      side.busy = true;
      side.result = null;
      side.error = null;
    });
    try {
      final request = await client.send(side.session!, input);
      if (!mounted) {
        await client.cancel(request);
        return;
      }
      setState(() {
        side.request = request;
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
    if (widget.readOnly) return;
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
    if (_polling || _client == null || !mounted) return;
    _polling = true;
    try {
      for (final side in [_left, _right]) {
        if (!side.pending || side.busy || side.request == null) continue;
        final request = side.request!;
        try {
          final progress =
              await _client!.poll(request, side.runtime!, side.session!);
          if (!mounted) return;
          if (side.request != request || !side.pending) continue;
          setState(() {
            side.state = progress.state;
            side.result = progress.result;
            side.error = null;
            side.pending = progress.result == null;
          });
        } catch (_) {
          if (mounted) {
            setState(() {
              side.error = '応答を取得できません。接続・監査を確認してください。本文の表示は保留しています。';
              side.result = null;
            });
          }
        }
      }
    } finally {
      _polling = false;
    }
  }

  Future<void> _cancel(_Conversation side) async {
    if (!side.pending || side.request == null || side.busy) return;
    setState(() => side.busy = true);
    try {
      await _client!.cancel(side.request!);
      if (mounted) {
        setState(() {
          side.pending = false;
          side.result = null;
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
    if (side.busy || side.pending || side.session == null) return;
    setState(() => side.busy = true);
    try {
      await _client!.close(side.session!);
      if (!mounted) return;
      setState(() {
        side.session = null;
        side.request = null;
        side.result = null;
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
    final active = widget.readOnly ||
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
        const SizedBox(height: 8),
        const Text('送信後はownerの承認を待ちます。中止は応答の採用を止めますが、実行系の処理停止や巻戻しを保証しません。'),
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
                  onPressed: _connecting ? null : _connect,
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
                        : (v) => setState(() => side.runtime = v)),
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
                      onPressed: !widget.readOnly && side.pending && !side.busy
                          ? () => _cancel(side)
                          : null,
                      child: const Text('中止')),
                ]),
                Text('状態: ${side.state}'),
                if (side.session != null)
                  SelectableText('セッション: ${side.session}'),
                if (side.request != null) SelectableText('要求: ${side.request}'),
                if (side.state == '承認待ち') const Text('ownerの承認操作を待っています。'),
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
                ],
              ],
            )));
  }
}

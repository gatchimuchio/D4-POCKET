import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

import '../services/a2a_connection_client.dart';
import 'shared.dart';

class A2aConnectionCenter extends StatefulWidget {
  const A2aConnectionCenter({
    super.key,
    required this.transport,
    this.onPrepareAgentTask,
  });

  final BrokerTransport? transport;
  final ValueChanged<A2aTaskQueryResult>? onPrepareAgentTask;

  @override
  State<A2aConnectionCenter> createState() => _A2aConnectionCenterState();
}

class _A2aConnectionCenterState extends State<A2aConnectionCenter> {
  final _agentIdController = TextEditingController();
  final _agentCardUriController = TextEditingController();
  final _interfaceUriController = TextEditingController();
  final _messageController = TextEditingController();
  late final A2aConnectionClient? _client;
  List<A2aConnectionSummary> _connections = const [];
  bool _loading = false;
  bool _connecting = false;
  bool _sendingMessage = false;
  bool _gettingTask = false;
  String? _selectedAgentId;
  String? _message;
  String? _sendMessageStatus;
  String? _pendingTaskAgentId;
  String? _pendingTaskInterfaceUri;
  String? _pendingTaskInterfaceHash;
  String? _pendingTaskId;
  A2aTaskQueryResult? _taskQueryResult;
  A2aStreamSendResult? _streamResult;

  @override
  void initState() {
    super.initState();
    final transport = widget.transport;
    _client = transport == null ? null : A2aConnectionClient(transport);
    if (_client != null) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) _loadConnections();
      });
    }
  }

  @override
  void dispose() {
    _agentIdController.dispose();
    _agentCardUriController.dispose();
    _interfaceUriController.dispose();
    _messageController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final client = _client;
    final sendableConnections = _connections
        .where((connection) =>
            connection.connectionState == 'connected' &&
            connection.supportedInterfaces.any((interface) =>
                interface.status == 'supported' &&
                interface.binding == 'jsonrpc' &&
                interface.protocolVersion == '1.0'))
        .toList(growable: false);
    final selectedAgentId = sendableConnections.any(
      (connection) => connection.agentId == _selectedAgentId,
    )
        ? _selectedAgentId
        : null;
    final selectedConnection = selectedAgentId == null
        ? null
        : sendableConnections.firstWhere(
            (connection) => connection.agentId == selectedAgentId,
          );
    final supportsNativeStream =
        defaultTargetPlatform == TargetPlatform.windows ||
            defaultTargetPlatform == TargetPlatform.linux;
    final supportsNativeTaskQuery =
        defaultTargetPlatform == TargetPlatform.windows ||
            defaultTargetPlatform == TargetPlatform.linux;
    return ShellPage(
      title: 'A2A接続センター',
      evidenceTitle: 'A2A Connection Center',
      children: [
        const BorderedPanel(
          child: Text(
            'Agent Cardは未信頼の外部metadataとして扱います。HTTPはloopback IPv4に限定し、'
            'HTTPSは証明書を検証しpublic IPへ接続します。接続と各Message送信はRust Broker経由でnative Owner確認を行います。'
            'Message送信と手動Task状態照会は無認証のA2A v1.0 JSON-RPC interfaceだけに対応し、Credential値を扱いません。'
            'Trust・Permission・Approval・Authorityは接続metadataから生成しません。Taskの履歴・Artifact本文は通常照会では表示しません。'
            '完了Taskのtext Artifactだけ、Windows/Linuxで毎回native Owner確認を経て上限付き全文表示できます。file/dataは未対応で保存しません。',
          ),
        ),
        if (client == null)
          const BorderedPanel(
            child: Text('Broker接続がないためA2A接続と一覧取得は利用できません。'),
          )
        else ...[
          BorderedPanel(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('Agent Card登録',
                    style: Theme.of(context).textTheme.titleMedium),
                const SizedBox(height: 8),
                TextField(
                  controller: _agentIdController,
                  enabled: !_loading && !_connecting && !_gettingTask,
                  maxLength: 128,
                  decoration: const InputDecoration(
                    border: OutlineInputBorder(),
                    labelText: 'Agent識別子（Agent ID）',
                  ),
                ),
                const SizedBox(height: 8),
                TextField(
                  controller: _agentCardUriController,
                  enabled: !_loading && !_connecting && !_gettingTask,
                  maxLength: 2048,
                  autocorrect: false,
                  enableSuggestions: false,
                  decoration: const InputDecoration(
                    border: OutlineInputBorder(),
                    labelText: 'Agent Card接続先URI（http://127.x.x.x:port/...）',
                    helperText: 'HTTPS、外部host、query、fragment、userinfoは受け付けません。',
                  ),
                ),
                const SizedBox(height: 4),
                const Text(
                  '再起動後に「再確認が必要」と表示されたreceiptは、同じAgent IDと現在のAgent Card URIを入力し、native Owner確認後に再検証できます。過去の承認は再利用しません。',
                ),
                const SizedBox(height: 8),
                Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  children: [
                    FilledButton.icon(
                      onPressed: _loading || _connecting || _gettingTask
                          ? null
                          : _connect,
                      icon: const Icon(Icons.link),
                      label: Text(_connecting ? '接続中…' : 'Owner確認して接続'),
                    ),
                    OutlinedButton.icon(
                      onPressed: _loading || _connecting || _gettingTask
                          ? null
                          : _loadConnections,
                      icon: const Icon(Icons.refresh),
                      label: const Text('一覧を更新'),
                    ),
                  ],
                ),
                if (_message != null) ...[
                  const SizedBox(height: 12),
                  Semantics(liveRegion: true, child: Text(_message!)),
                ],
              ],
            ),
          ),
          BorderedPanel(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('A2A Message送信',
                    style: Theme.of(context).textTheme.titleMedium),
                const SizedBox(height: 6),
                const Text(
                  '各送信を個別にnative Owner確認します。Agent ID・interface hash・本文hashへ束縛した一回限りPermissionを消費します。応答が不明な場合は再送しません。',
                ),
                const SizedBox(height: 8),
                if (sendableConnections.isEmpty)
                  const Text('送信可能な無認証 JSON-RPC v1.0 interfaceが登録されていません。')
                else ...[
                  DropdownButtonFormField<String>(
                    initialValue: selectedAgentId,
                    decoration: const InputDecoration(
                      border: OutlineInputBorder(),
                      labelText: '送信先Agent',
                    ),
                    items: [
                      for (final connection in sendableConnections)
                        DropdownMenuItem(
                          value: connection.agentId,
                          child: Text(
                            '${connection.displayName} (${connection.agentId})',
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                    ],
                    onChanged: _sendingMessage ||
                            _connecting ||
                            _loading ||
                            _gettingTask
                        ? null
                        : (value) => setState(() {
                              _selectedAgentId = value;
                              _streamResult = null;
                              if (_pendingTaskAgentId != value) {
                                _clearPendingTask();
                              }
                            }),
                  ),
                  const SizedBox(height: 8),
                  TextField(
                    controller: _interfaceUriController,
                    enabled: !_sendingMessage &&
                        !_connecting &&
                        !_loading &&
                        !_gettingTask,
                    maxLength: 2048,
                    autocorrect: false,
                    enableSuggestions: false,
                    decoration: const InputDecoration(
                      border: OutlineInputBorder(),
                      labelText: 'Agent CardのsupportedInterfaces.url',
                      helperText:
                          '登録receiptのinterface hashと完全一致するURIを指定します。HTTPはloopback IPv4のみ。',
                    ),
                  ),
                  const SizedBox(height: 8),
                  TextField(
                    controller: _messageController,
                    enabled: !_sendingMessage &&
                        !_connecting &&
                        !_loading &&
                        !_gettingTask,
                    maxLength: 1024,
                    minLines: 2,
                    maxLines: 6,
                    decoration: const InputDecoration(
                      border: OutlineInputBorder(),
                      labelText: '送信Message（最大1024文字 / UTF-8 4 KiB）',
                    ),
                  ),
                  const SizedBox(height: 8),
                  FilledButton.icon(
                    onPressed: selectedAgentId == null ||
                            _sendingMessage ||
                            _connecting ||
                            _loading ||
                            _gettingTask
                        ? null
                        : _sendMessage,
                    icon: _sendingMessage
                        ? const SizedBox.square(
                            dimension: 16,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.send),
                    label:
                        Text(_sendingMessage ? '送信中…' : 'native Owner確認して一回送信'),
                  ),
                  const SizedBox(height: 8),
                  OutlinedButton.icon(
                    key: const ValueKey('a2a-send-stream'),
                    onPressed: selectedAgentId == null ||
                            selectedConnection?.streaming != true ||
                            !supportsNativeStream ||
                            _sendingMessage ||
                            _connecting ||
                            _loading ||
                            _gettingTask
                        ? null
                        : _sendStream,
                    icon: const Icon(Icons.stream),
                    label: const Text('native Owner確認してStreamを一回送信'),
                  ),
                  if (!supportsNativeStream)
                    const Text(
                        '現行A2A Stream native Owner経路はWindows/Linux Desktopのみ対応です。'),
                  if (selectedConnection?.streaming != true)
                    const Text('選択AgentはAgent Cardでstreamingを宣言していません。'),
                ],
                if (_sendMessageStatus != null) ...[
                  const SizedBox(height: 12),
                  Semantics(liveRegion: true, child: Text(_sendMessageStatus!)),
                ],
                if (_streamResult != null) ...[
                  const SizedBox(height: 8),
                  BorderedPanel(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(_streamResult!.status == 'unknown'
                            ? 'Stream結果不明（自動再接続・再送なし）'
                            : _streamResult!.streamKind == 'message'
                                ? 'Message-only streamを受理（Task完了を意味しません）'
                                : 'Task stream終端を観測（外部Taskの状態表示のみ）'),
                        Text(
                          'state=${_streamResult!.state ?? '該当なし'} / events=${_streamResult!.eventCount ?? 'unknown'} / result hash=${_streamResult!.resultHash ?? 'unknown'} / Audit=${_streamResult!.auditEventId}',
                        ),
                        for (final event in _streamResult!.events)
                          Text(
                            '${event.kind}: state=${event.state ?? '該当なし'}, parts=${event.partCount ?? '該当なし'}, hash=${event.eventHash}',
                          ),
                      ],
                    ),
                  ),
                ],
                if (_pendingTaskId != null) ...[
                  const SizedBox(height: 8),
                  Text('送信後のTask ID（memory-only）: $_pendingTaskId'),
                  if (supportsNativeTaskQuery)
                    OutlinedButton.icon(
                      onPressed: _gettingTask ||
                              _loading ||
                              _connecting ||
                              _sendingMessage
                          ? null
                          : _getTask,
                      icon: _gettingTask
                          ? const SizedBox.square(
                              dimension: 16,
                              child: CircularProgressIndicator(strokeWidth: 2),
                            )
                          : const Icon(Icons.query_stats),
                      label: Text(_gettingTask
                          ? '状態取得中…'
                          : 'native Owner確認してTask状態・Artifact一覧を一回取得'),
                    )
                  else
                    const Text('macOSの現行A2A native Owner契約ではTask取得は未対応です。'),
                  if (_taskQueryResult?.status == 'accepted') ...[
                    const SizedBox(height: 8),
                    Text(
                      'Task state=${_taskQueryResult!.state ?? 'unknown'} / history=0 / response hash=${_taskQueryResult!.resultHash ?? 'unknown'}',
                    ),
                    for (final artifact in _taskQueryResult!.artifacts) ...[
                      const SizedBox(height: 8),
                      Text(
                        'Artifact ID=${artifact.artifactId} / hash=${artifact.artifactHash} / parts=${artifact.partCount} / text bytes=${artifact.textBytes?.toString() ?? 'unknown'}',
                      ),
                      if (artifact.textOnly &&
                          artifact.textBytes != null &&
                          _taskQueryResult!.state == 'completed' &&
                          supportsNativeTaskQuery)
                        OutlinedButton.icon(
                          onPressed: _gettingTask ||
                                  _loading ||
                                  _connecting ||
                                  _sendingMessage
                              ? null
                              : () => _getArtifact(artifact),
                          icon: const Icon(Icons.visibility),
                          label: const Text(
                              'native Owner確認してこのtext Artifactを全文表示'),
                        )
                      else
                        const Text('全文表示不可（Task未完了、非text、危険文字、または64 KiB上限超過）。'),
                    ],
                    if (_taskQueryResult!.selectedArtifact case final selected?)
                      BorderedPanel(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            const Text('外部Artifact本文（未信頼・全文表示）'),
                            Text(
                              'Artifact hash=${selected.artifactHash} / content hash=${selected.contentHash} / ${selected.byteLength} bytes',
                            ),
                            for (final part in selected.parts) ...[
                              const SizedBox(height: 8),
                              SelectableText(part),
                            ],
                          ],
                        ),
                      ),
                    if (widget.onPrepareAgentTask != null &&
                        _taskQueryResult!.selectedArtifact != null) ...[
                      const Text(
                        'Agent Centerへ渡す操作はTask dataの移送だけです。新規TaskのPermission／Approvalを引き継がず、ここではTaskを実行しません。',
                      ),
                      OutlinedButton.icon(
                        key: const ValueKey(
                            'handoff-a2a-artifact-to-agent-center'),
                        onPressed:
                            _gettingTask ? null : _prepareAgentTaskFromArtifact,
                        icon: const Icon(Icons.call_made),
                        label: const Text('選択ArtifactをAgent Centerの新規Taskへ渡す'),
                      ),
                    ],
                  ],
                ],
              ],
            ),
          ),
          BorderedPanel(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Expanded(
                      child: Text('接続metadata',
                          style: Theme.of(context).textTheme.titleMedium),
                    ),
                    if (_loading)
                      const SizedBox.square(
                        dimension: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      ),
                  ],
                ),
                const SizedBox(height: 8),
                if (_connections.isEmpty && !_loading)
                  const Text('Broker一覧にA2A接続metadataはありません。')
                else
                  for (final connection in _connections)
                    _A2aConnectionTile(connection: connection),
              ],
            ),
          ),
        ],
      ],
    );
  }

  Future<void> _loadConnections() async {
    final client = _client;
    if (client == null || _loading || _connecting || _gettingTask) return;
    setState(() {
      _loading = true;
      _message = null;
    });
    try {
      final connections = await client.list();
      if (!mounted) return;
      setState(() {
        _connections = connections;
        if (!_connections.any((connection) =>
            connection.agentId == _selectedAgentId &&
            connection.supportedInterfaces.any((interface) =>
                interface.status == 'supported' &&
                interface.binding == 'jsonrpc' &&
                interface.protocolVersion == '1.0'))) {
          _selectedAgentId = null;
        }
        if (_pendingTaskAgentId != null &&
            !connections.any((connection) =>
                connection.agentId == _pendingTaskAgentId &&
                connection.connectionState == 'connected' &&
                connection.supportedInterfaces.any((interface) =>
                    interface.status == 'supported' &&
                    interface.interfaceHash == _pendingTaskInterfaceHash))) {
          _clearPendingTask();
        }
        _message = 'Brokerのmetadata-only一覧を更新しました。';
      });
    } on Object {
      if (!mounted) return;
      setState(() {
        _message = 'Broker一覧を取得できませんでした。接続状態を成功扱いにはしていません。';
      });
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _connect() async {
    final client = _client;
    if (client == null || _loading || _connecting || _gettingTask) return;
    setState(() {
      _connecting = true;
      _message = '要求をBrokerへ送信しています。native Owner確認後に接続します。';
    });
    try {
      final receipt = await client.connect(
        agentId: _agentIdController.text,
        agentCardUri: _agentCardUriController.text,
      );
      if (!mounted) return;
      setState(() {
        _message =
            '接続metadataを受理しました。Trustは未審査のままです。Audit=${receipt.connectionAuditId}';
      });
      try {
        final connections = await client.list();
        if (!mounted) return;
        setState(() => _connections = connections);
      } on Object {
        if (!mounted) return;
        setState(() {
          _message =
              '接続receiptは受理されましたが一覧を更新できません。Audit=${receipt.connectionAuditId}';
        });
      }
    } on Object {
      if (!mounted) return;
      setState(() {
        _message = 'A2A接続を確認できませんでした。受理receiptがないため成功表示はしていません。';
      });
    } finally {
      if (mounted) {
        _agentCardUriController.clear();
        setState(() => _connecting = false);
      }
    }
  }

  Future<void> _sendMessage() async {
    final client = _client;
    final agentId = _selectedAgentId;
    if (client == null ||
        agentId == null ||
        _loading ||
        _connecting ||
        _gettingTask ||
        _sendingMessage) {
      return;
    }
    setState(() {
      _sendingMessage = true;
      _streamResult = null;
      _clearPendingTask();
      _sendMessageStatus = '要求をBrokerへ送信しています。native確認後、一回だけ実行します。';
    });
    final requestedInterfaceUri = _interfaceUriController.text;
    try {
      final result = await client.sendMessage(
        agentId: agentId,
        interfaceUri: _interfaceUriController.text,
        message: _messageController.text,
      );
      if (!mounted) return;
      setState(() {
        _taskQueryResult = null;
        _sendMessageStatus = result.status == 'unknown'
            ? '送信結果は不明です。二重送信防止のため再送しません。Audit=${result.auditEventId}'
            : 'A2A Message応答を受理しました（Task完了ではありません）。kind=${result.resultKind} / state=${result.remoteState ?? 'unknown'} / id=${result.remoteId} / result hash=${result.resultHash} / Audit=${result.auditEventId}';
        if (result.status == 'accepted' &&
            result.resultKind == 'task' &&
            result.remoteId != null) {
          _pendingTaskAgentId = agentId;
          _pendingTaskInterfaceUri = requestedInterfaceUri;
          _pendingTaskInterfaceHash = result.interfaceHash;
          _pendingTaskId = result.remoteId;
        }
      });
    } on Object {
      if (!mounted) return;
      setState(() {
        _sendMessageStatus =
            'Brokerから確定応答を得られませんでした。外部作用の結果が不明な場合は再送せず、Auditを確認してください。';
      });
    } finally {
      _interfaceUriController.clear();
      _messageController.clear();
      if (mounted) setState(() => _sendingMessage = false);
    }
  }

  Future<void> _sendStream() async {
    final client = _client;
    final agentId = _selectedAgentId;
    if (client == null ||
        agentId == null ||
        _loading ||
        _connecting ||
        _gettingTask ||
        _sendingMessage ||
        !_connections.any((connection) =>
            connection.agentId == agentId &&
            connection.connectionState == 'connected' &&
            connection.streaming)) {
      return;
    }
    setState(() {
      _sendingMessage = true;
      _clearPendingTask();
      _streamResult = null;
      _sendMessageStatus =
          '要求をBrokerへ送信しています。native Owner確認後、一回だけ実行し、終端eventまたは期限まで読み取ります。';
    });
    try {
      final result = await client.sendStream(
        agentId: agentId,
        interfaceUri: _interfaceUriController.text,
        message: _messageController.text,
      );
      if (!mounted) return;
      setState(() {
        _streamResult = result;
        _taskQueryResult = null;
        _sendMessageStatus = result.status == 'unknown'
            ? 'Stream結果は不明です。重複作用防止のため再接続・再送しません。Audit=${result.auditEventId}'
            : 'Streamを一回処理しました。event本文とTask/Message IDは画面へ表示せず、Authorityも生成しません。Audit=${result.auditEventId}';
      });
    } on Object {
      if (!mounted) return;
      setState(() {
        _streamResult = null;
        _sendMessageStatus =
            'Stream結果を確定できませんでした。外部作用の結果が不明な場合は再接続・再送せず、Auditを確認してください。';
      });
    } finally {
      _interfaceUriController.clear();
      _messageController.clear();
      if (mounted) setState(() => _sendingMessage = false);
    }
  }

  Future<void> _getTask() async {
    final client = _client;
    final agentId = _pendingTaskAgentId;
    final interfaceUri = _pendingTaskInterfaceUri;
    final taskId = _pendingTaskId;
    if (client == null ||
        agentId == null ||
        interfaceUri == null ||
        taskId == null ||
        _gettingTask ||
        _loading ||
        _connecting ||
        _sendingMessage) {
      return;
    }
    setState(() {
      _gettingTask = true;
      _taskQueryResult = null;
      _sendMessageStatus =
          'Task状態・Artifact一覧をBrokerへ送信しています。native Owner確認後に一回だけ照会します。';
    });
    try {
      final result = await client.getTask(
        agentId: agentId,
        interfaceUri: interfaceUri,
        taskId: taskId,
      );
      if (!mounted) return;
      setState(() {
        _taskQueryResult = result.status == 'accepted' ? result : null;
        _sendMessageStatus = result.status == 'unknown'
            ? 'Task状態は不明です。自動再取得しません。手動再照会には新しいOwner確認が必要です。Audit=${result.auditEventId}'
            : 'Task metadataを取得しました。state=${result.state} / artifact count=${result.artifactCount} / history count=0 / response hash=${result.resultHash} / Audit=${result.auditEventId}';
      });
    } on Object {
      if (!mounted) return;
      setState(() {
        _sendMessageStatus =
            'Task状態を確定できませんでした。自動再取得しません。再照会には新しいnative Owner確認が必要です。';
      });
    } finally {
      if (mounted) setState(() => _gettingTask = false);
    }
  }

  Future<void> _getArtifact(A2aArtifactManifestItem artifact) async {
    final client = _client;
    final agentId = _pendingTaskAgentId;
    final interfaceUri = _pendingTaskInterfaceUri;
    final taskId = _pendingTaskId;
    if (client == null ||
        agentId == null ||
        interfaceUri == null ||
        taskId == null ||
        _taskQueryResult == null ||
        !_taskQueryResult!.artifacts.any((item) =>
            item.artifactId == artifact.artifactId &&
            item.artifactHash == artifact.artifactHash &&
            item.textOnly) ||
        _gettingTask ||
        _loading ||
        _connecting ||
        _sendingMessage) {
      return;
    }
    setState(() {
      _gettingTask = true;
      _taskQueryResult = null;
      _sendMessageStatus =
          '選択Artifactをhash照合してBrokerへ要求しています。native Owner確認後、text全文を一回だけ表示します。';
    });
    try {
      final result = await client.getTask(
        agentId: agentId,
        interfaceUri: interfaceUri,
        taskId: taskId,
        artifactId: artifact.artifactId,
        artifactHash: artifact.artifactHash,
      );
      if (!mounted) return;
      setState(() {
        _taskQueryResult = result;
        _sendMessageStatus =
            '選択Artifactを現在hashと照合し、揮発状態で全文表示しました。保存・Audit本文記録はありません。Audit=${result.auditEventId}';
      });
    } on Object {
      if (!mounted) return;
      setState(() {
        _taskQueryResult = null;
        _sendMessageStatus =
            'Artifactを安全に表示できませんでした。自動再取得しません。新しいOwner確認を含む明示操作が必要です。';
      });
    } finally {
      if (mounted) setState(() => _gettingTask = false);
    }
  }

  void _prepareAgentTaskFromArtifact() {
    final callback = widget.onPrepareAgentTask;
    final source = _taskQueryResult;
    if (callback == null ||
        source == null ||
        source.status != 'accepted' ||
        source.state != 'completed' ||
        source.selectedArtifact == null ||
        _gettingTask) {
      return;
    }
    setState(() {
      _taskQueryResult = null;
      _sendMessageStatus =
          '選択ArtifactをAgent Centerの揮発Task draftへ移しました。新規Taskは未検査・未実行です。';
    });
    callback(source);
  }

  void _clearPendingTask() {
    _pendingTaskAgentId = null;
    _pendingTaskInterfaceUri = null;
    _pendingTaskInterfaceHash = null;
    _pendingTaskId = null;
    _taskQueryResult = null;
  }
}

class _A2aConnectionTile extends StatelessWidget {
  const _A2aConnectionTile({required this.connection});

  final A2aConnectionSummary connection;

  @override
  Widget build(BuildContext context) {
    final restored = connection.connectionState == 'restored_pending_review';
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('表示名（未信頼metadata）: ${connection.displayName}',
                style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 4),
            Text('Agent識別子: ${connection.agentId}'),
            Text('説明（未信頼metadata）: ${connection.descriptionSummary}'),
            Text('版: ${connection.version}'),
            Text('接続先hash: ${connection.endpointHash}'),
            for (final interface in connection.supportedInterfaces) ...[
              Text(
                  'A2A接続方式: ${interface.binding} / Protocol版: ${interface.protocolVersion} / 通信方式: ${interface.transport} / 実行状態: ${interface.status}'),
              Text('接続方式照合値: ${interface.interfaceHash ?? '不明'}'),
              Text('状態理由（未信頼metadata）: ${interface.reason}'),
            ],
            Text('接続状態: ${restored ? '再確認が必要' : '接続済み'}'),
            Text(
                '証拠: ${connection.evidenceSource} / 承認: ${connection.approvalState}'),
            Text('信頼状態: 未審査（pending_review）— ${connection.trustReason}'),
            Text('接続Audit: ${connection.connectionAuditId}'),
          ],
        ),
      ),
    );
  }
}

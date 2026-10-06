import 'dart:convert';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException;

import '../models/generated_contracts.dart';
import '../services/ai_edit_client.dart';
import '../services/compose_client.dart';
import '../services/export_client.dart';
import '../services/profile_client.dart';
import '../services/shell_core_client.dart';
import '../services/update_client.dart';
import 'mcp_connection_center.dart';
import 'shared.dart';

const guiShellProductVersion = String.fromEnvironment(
  'GUI_SHELL_PRODUCT_VERSION',
  defaultValue: '開発版',
);

class SettingsScreen extends StatefulWidget {
  const SettingsScreen({
    super.key,
    required this.client,
    this.active = true,
    this.requestApplicationExit,
    this.focusUpdateCenterRequest = 0,
    this.focusMcpCenterRequest = 0,
    this.focusComposeCenterRequest = 0,
    this.focusExportCenterRequest = 0,
  });

  final ShellCoreClient client;
  final bool active;
  final Future<ui.AppExitResponse> Function(ui.AppExitType)?
      requestApplicationExit;
  final int focusUpdateCenterRequest;
  final int focusMcpCenterRequest;
  final int focusComposeCenterRequest;
  final int focusExportCenterRequest;

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  final GlobalKey _updateCenterKey = GlobalKey();
  final GlobalKey _composeCenterKey = GlobalKey();
  final GlobalKey _exportCenterKey = GlobalKey();
  final Set<String> _selectedExportModules =
      Set<String>.of(guiShellOptionalExportModules.keys);
  final Set<String> _stagedUpdateCandidates = <String>{};
  final TextEditingController _searchController = TextEditingController();
  bool _modifiedOnly = false;
  bool _authorityOnly = false;
  bool _dangerousOnly = false;
  bool _phaseReleaseOnly = false;
  late final ProfileClient? _profileClient;
  late final ComposeClient? _composeClient;
  late final ExportClient? _exportClient;
  late final AiEditClient? _aiEditClient;
  late final UpdateClient? _updateClient;
  Future<List<Map<String, Object?>>>? _profilesFuture;
  Future<Map<String, Object?>>? _updatesFuture;
  final TextEditingController _profileIdController =
      TextEditingController(text: 'default.local');
  final TextEditingController _profileNameController =
      TextEditingController(text: '標準ローカル実行');
  String? _profileMessage;
  String? _exportedProfile;
  String? _composeMessage;
  String? _composedManifest;
  String? _previewJson;
  String? _exportMessage;
  String? _exportReceipt;
  bool _exportInProgress = false;
  String? _aiEditMessage;
  String? _aiEditReceipt;
  String? _updateMessage;
  bool _checkingUpdateCatalog = false;
  bool _updateOperationInProgress = false;
  final TextEditingController _composeIdController =
      TextEditingController(text: 'd4-pocket-local');
  final TextEditingController _composeNameController =
      TextEditingController(text: 'D4 Pocket ローカル構成');
  final TextEditingController _composeRuntimeIdsController =
      TextEditingController();
  final TextEditingController _composeAgentIdsController =
      TextEditingController();
  final TextEditingController _composeToolIdsController =
      TextEditingController();
  final TextEditingController _composeMcpIdsController =
      TextEditingController();
  final TextEditingController _composeCapabilityRequirementsController =
      TextEditingController(text: 'runtime.read\nagent.metadata');
  final TextEditingController _exportUpdatePublicKeyController =
      TextEditingController();
  final TextEditingController _exportUpdateSourceController =
      TextEditingController();
  String _exportUpdateChannel = 'stable';
  String _composeThemeMode = 'system';
  String _composeDensity = 'comfortable';
  String _composeContentVisibility = 'summary';
  Map<String, Object?>? _lastAcceptedComposeManifest;
  final TextEditingController _editInstructionController =
      TextEditingController(text: '構成Preview結果へ対象platformを表示する');
  final TextEditingController _editTargetPathController = TextEditingController(
      text: 'apps/desktop_flutter/lib/screens/settings.dart');

  @override
  void dispose() {
    _searchController.dispose();
    _profileIdController.dispose();
    _profileNameController.dispose();
    _composeIdController.dispose();
    _composeNameController.dispose();
    _composeRuntimeIdsController.dispose();
    _composeAgentIdsController.dispose();
    _composeToolIdsController.dispose();
    _composeMcpIdsController.dispose();
    _composeCapabilityRequirementsController.dispose();
    _exportUpdatePublicKeyController.dispose();
    _exportUpdateSourceController.dispose();
    _editInstructionController.dispose();
    _editTargetPathController.dispose();
    super.dispose();
  }

  @override
  void didUpdateWidget(covariant SettingsScreen oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!oldWidget.active && widget.active) {
      _updatesFuture = _updateClient?.list();
    }
    if (oldWidget.focusUpdateCenterRequest != widget.focusUpdateCenterRequest) {
      _focusSettingsSection(_updateCenterKey);
    }
    if (oldWidget.focusComposeCenterRequest !=
        widget.focusComposeCenterRequest) {
      _focusSettingsSection(_composeCenterKey);
    }
    if (oldWidget.focusExportCenterRequest != widget.focusExportCenterRequest) {
      _focusSettingsSection(_exportCenterKey);
    }
  }

  void _focusSettingsSection(GlobalKey key) {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final targetContext = key.currentContext;
      if (!mounted || targetContext == null) return;
      Scrollable.ensureVisible(
        targetContext,
        alignment: 0.08,
        duration: const Duration(milliseconds: 220),
        curve: Curves.easeOutCubic,
      );
    });
  }

  @override
  void initState() {
    super.initState();
    _profileClient = widget.client.brokerTransport == null
        ? null
        : ProfileClient(widget.client.brokerTransport!);
    _composeClient = widget.client.brokerTransport == null
        ? null
        : ComposeClient(widget.client.brokerTransport!);
    _exportClient = widget.client.brokerTransport == null
        ? null
        : ExportClient(widget.client.brokerTransport!);
    _aiEditClient = widget.client.brokerTransport == null
        ? null
        : AiEditClient(widget.client.brokerTransport!);
    _updateClient = widget.client.brokerTransport == null
        ? null
        : UpdateClient(widget.client.brokerTransport!);
    _refreshProfiles();
    _updatesFuture = widget.active ? _updateClient?.list() : null;
  }

  @override
  Widget build(BuildContext context) {
    final settings = widget.client.getSnapshot().settings;
    final filtered = settings.where(_matchesFilters).toList();
    return ShellPage(
      title: '設定',
      children: [
        const BorderedPanel(
          child: Text('D4 Pocket 製品版: $guiShellProductVersion'),
        ),
        BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('検索／絞込み', style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 8),
              TextField(
                controller: _searchController,
                decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  prefixIcon: Icon(Icons.search),
                  labelText: '設定を検索',
                ),
                onChanged: (_) => setState(() {}),
              ),
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  FilterChip(
                    label: const Text('変更済み'),
                    selected: _modifiedOnly,
                    onSelected: (value) =>
                        setState(() => _modifiedOnly = value),
                  ),
                  FilterChip(
                    label: const Text('権限関連'),
                    selected: _authorityOnly,
                    onSelected: (value) =>
                        setState(() => _authorityOnly = value),
                  ),
                  FilterChip(
                    label: const Text('危険'),
                    selected: _dangerousOnly,
                    onSelected: (value) =>
                        setState(() => _dangerousOnly = value),
                  ),
                  FilterChip(
                    label: const Text('段階／リリース'),
                    selected: _phaseReleaseOnly,
                    onSelected: (value) =>
                        setState(() => _phaseReleaseOnly = value),
                  ),
                ],
              ),
            ],
          ),
        ),
        const SectionList(
          title: '権限境界',
          rows: [
            '段階Bの仕上げでは設定を表示専用とします。',
            '初期化、書き出し、変更操作はShell Coreの承認経路を通す必要があります。',
            '危険または権限関連の設定には、操作者確認用の印を付けます。',
          ],
        ),
        McpConnectionCenterPanel(
          transport: widget.client.brokerTransport,
          focusRequest: widget.focusMcpCenterRequest,
        ),
        _profilePanel(),
        _composePanel(),
        _exportPanel(),
        _aiEditPanel(),
        KeyedSubtree(key: _updateCenterKey, child: _updatePanel()),
        if (filtered.isEmpty)
          const EmptyStatePanel(
            title: '一致する設定なし',
            meaning: '現在の絞込み条件に一致する設定射影がスナップショット内にありません。',
            phaseBBlocked: false,
            nextAction: '絞込みを解除するか、設定が変わった場合は診断を更新してください。',
          )
        else
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: DataTable(
              columns: const [
                DataColumn(label: Text('設定')),
                DataColumn(label: Text('現在値')),
                DataColumn(label: Text('出所')),
                DataColumn(label: Text('有効値／注記')),
                DataColumn(label: Text('印')),
              ],
              rows: [for (final setting in filtered) _settingRow(setting)],
            ),
          ),
      ],
    );
  }

  Widget _composePanel() {
    final client = _composeClient;
    if (client == null) {
      return BorderedPanel(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('GUI Shell構成',
                key: _composeCenterKey,
                style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 4),
            const Text(
                'Broker接続がないためManifestを生成できません。local snapshotは構成権限の根拠ではありません。'),
          ],
        ),
      );
    }
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('GUI Shell構成',
              key: _composeCenterKey,
              style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 4),
          const Text(
            'Runtime、Agent、Tool、MCP、Theme、Capability、SettingsをManifestへまとめます。build、独立App identity、Credential、Permission、Approval、Audit chainは生成・継承しません。',
          ),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              SizedBox(
                width: 180,
                child: TextField(
                  controller: _composeIdController,
                  decoration: const InputDecoration(labelText: '構成識別子'),
                ),
              ),
              SizedBox(
                width: 240,
                child: TextField(
                  controller: _composeNameController,
                  decoration: const InputDecoration(labelText: '構成表示名'),
                ),
              ),
              FilledButton.icon(
                key: const ValueKey('compose-manifest-button'),
                onPressed: () => _runCompose(client),
                icon: const Icon(Icons.account_tree_outlined),
                label: const Text('構成Manifest作成'),
              ),
              OutlinedButton.icon(
                key: const ValueKey('compose-preview-button'),
                onPressed: () => _runPreview(client),
                icon: const Icon(Icons.preview_outlined),
                label: const Text('構成Preview'),
              ),
            ],
          ),
          const SizedBox(height: 8),
          const Text(
            '各欄は1行につき1つのManifest参照IDです。候補一覧の取得や実在性・接続・信頼の確認は行いません。IDやCapability要件は権限を付与せず、重複などの契約検証はBrokerが行います。',
          ),
          const SizedBox(height: 8),
          TextField(
            key: const ValueKey('compose-runtime-ids'),
            controller: _composeRuntimeIdsController,
            minLines: 1,
            maxLines: 3,
            decoration: const InputDecoration(
              border: OutlineInputBorder(),
              labelText: 'Runtime参照ID（任意・1行に1つ）',
            ),
          ),
          const SizedBox(height: 8),
          TextField(
            key: const ValueKey('compose-agent-ids'),
            controller: _composeAgentIdsController,
            minLines: 1,
            maxLines: 3,
            decoration: const InputDecoration(
              border: OutlineInputBorder(),
              labelText: 'Agent参照ID（任意・1行に1つ）',
            ),
          ),
          const SizedBox(height: 8),
          TextField(
            key: const ValueKey('compose-tool-ids'),
            controller: _composeToolIdsController,
            minLines: 1,
            maxLines: 3,
            decoration: const InputDecoration(
              border: OutlineInputBorder(),
              labelText: 'Tool参照ID（任意・1行に1つ）',
            ),
          ),
          const SizedBox(height: 8),
          TextField(
            key: const ValueKey('compose-mcp-ids'),
            controller: _composeMcpIdsController,
            minLines: 1,
            maxLines: 3,
            decoration: const InputDecoration(
              border: OutlineInputBorder(),
              labelText: 'MCP接続参照ID（任意・1行に1つ）',
            ),
          ),
          const SizedBox(height: 8),
          const Text('UI構成（Manifest上の要求。実際の権限は生成しません）'),
          Wrap(
            spacing: 12,
            runSpacing: 8,
            children: [
              SizedBox(
                width: 190,
                child: DropdownButtonFormField<String>(
                  key: const ValueKey('compose-theme-mode'),
                  initialValue: _composeThemeMode,
                  isExpanded: true,
                  decoration: const InputDecoration(
                    labelText: 'Theme表示',
                    border: OutlineInputBorder(),
                  ),
                  items: const [
                    DropdownMenuItem(value: 'system', child: Text('OSに合わせる')),
                    DropdownMenuItem(value: 'light', child: Text('明るい')),
                    DropdownMenuItem(value: 'dark', child: Text('暗い')),
                  ],
                  onChanged: (value) {
                    if (value != null) {
                      setState(() => _composeThemeMode = value);
                    }
                  },
                ),
              ),
              SizedBox(
                width: 190,
                child: DropdownButtonFormField<String>(
                  key: const ValueKey('compose-density'),
                  initialValue: _composeDensity,
                  isExpanded: true,
                  decoration: const InputDecoration(
                    labelText: '表示密度',
                    border: OutlineInputBorder(),
                  ),
                  items: const [
                    DropdownMenuItem(value: 'compact', child: Text('コンパクト')),
                    DropdownMenuItem(value: 'comfortable', child: Text('標準')),
                  ],
                  onChanged: (value) {
                    if (value != null) setState(() => _composeDensity = value);
                  },
                ),
              ),
              SizedBox(
                width: 220,
                child: DropdownButtonFormField<String>(
                  key: const ValueKey('compose-content-visibility'),
                  initialValue: _composeContentVisibility,
                  isExpanded: true,
                  decoration: const InputDecoration(
                    labelText: '内容表示要求',
                    border: OutlineInputBorder(),
                  ),
                  items: const [
                    DropdownMenuItem(value: 'none', child: Text('内容を表示しない')),
                    DropdownMenuItem(value: 'hash_only', child: Text('hashのみ')),
                    DropdownMenuItem(value: 'summary', child: Text('要約')),
                    DropdownMenuItem(value: 'redacted', child: Text('伏字を含む表示')),
                    DropdownMenuItem(
                      value: 'full',
                      child: Text('全文（実行時の許可は別途必要）'),
                    ),
                  ],
                  onChanged: (value) {
                    if (value != null) {
                      setState(() => _composeContentVisibility = value);
                    }
                  },
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          TextField(
            key: const ValueKey('compose-capability-requirements'),
            controller: _composeCapabilityRequirementsController,
            minLines: 1,
            maxLines: 3,
            decoration: const InputDecoration(
              border: OutlineInputBorder(),
              labelText: '機能要件ID（任意・1行に1つ）',
              helperText: '要求の記録だけを行い、PermissionやAuthorityは生成しません。',
            ),
          ),
          const SizedBox(height: 8),
          const SectionList(
            title: '固定・継承境界',
            rows: [
              'Theme識別子: d4-pocket / Locale: ja-JP',
              '継承禁止: Authority、Permission、Approval、Credential、Audit chainはすべてなし',
            ],
          ),
          if (_composeMessage != null) ...[
            const SizedBox(height: 8),
            Text(_composeMessage!),
          ],
          if (_composedManifest != null) ...[
            const SizedBox(height: 8),
            SelectableText(_composedManifest!),
          ],
          if (_previewJson != null) ...[
            const SizedBox(height: 8),
            const Text('構成Preview結果'),
            SelectableText(_previewJson!),
          ],
        ],
      ),
    );
  }

  Map<String, Object?> _composeManifest() => buildComposeManifestDraft(
        composeId: _composeIdController.text,
        displayName: _composeNameController.text,
        runtimeIds: _composeRuntimeIdsController.text,
        agentIds: _composeAgentIdsController.text,
        toolIds: _composeToolIdsController.text,
        mcpConnectionIds: _composeMcpIdsController.text,
        themeMode: _composeThemeMode,
        density: _composeDensity,
        contentVisibility: _composeContentVisibility,
        capabilityRequirements: _composeCapabilityRequirementsController.text,
      );

  Future<void> _runCompose(ComposeClient client) async {
    try {
      final receipt = await client.compose(_composeManifest());
      final manifest = receipt['compose_manifest'];
      _lastAcceptedComposeManifest =
          manifest is Map ? Map<String, Object?>.from(manifest) : null;
      _composedManifest = _lastAcceptedComposeManifest == null
          ? null
          : composeJson(_lastAcceptedComposeManifest!);
      _setComposeMessage('Manifestだけを作成しました。buildとApp identity生成は未実行です。');
    } catch (error) {
      _setComposeMessage('GUI Shell構成に失敗しました: $error');
    }
  }

  Future<void> _runPreview(ComposeClient client) async {
    try {
      final receipt = await client.preview(
        currentManifest: _lastAcceptedComposeManifest,
        candidateManifest: _composeManifest(),
      );
      _previewJson = composeJson(receipt);
      _setComposeMessage(
          '構成、機能要件、差分、版rollbackをPreviewしました。build、Export、rollbackは未実行です。');
      if (mounted) setState(() {});
    } catch (error) {
      _setComposeMessage('GUI Shell構成Previewに失敗しました: $error');
    }
  }

  Widget _exportPanel() {
    final client = _exportClient;
    if (client == null) {
      return BorderedPanel(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('GUI Shell Windows書出し',
                key: _exportCenterKey,
                style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 4),
            const Text('Broker接続がないため操作できません。'),
          ],
        ),
      );
    }
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('GUI Shell Windows書出し',
              key: _exportCenterKey,
              style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 4),
          const Text(
              '切出し対象の任意画面をManifest計画へ記録できます。権限・承認・監査・復旧などの必須境界は常に保持します。Owner確認後、独立Manifest fileを固定保存先へ作成します。画面選択は計画だけで、実binaryからの除去や実行可能App、build、Installer、署名は行いません。Credential・Permission・Approval・Audit chainは継承しません。キャンセルはBrokerに拒否として監査記録されます。'),
          const SizedBox(height: 8),
          ExpansionTile(
            key: const ValueKey('export-update-trust-advanced'),
            tilePadding: EdgeInsets.zero,
            title: const Text('更新署名trust（上級者向け）'),
            subtitle: const Text('空欄なら未構成。公開鍵とHTTPS配布元はnative Owner確認へ表示します。'),
            children: [
              const Align(
                alignment: Alignment.centerLeft,
                child: Text(
                    '秘密署名鍵は入力しません。公開鍵trustはExport Manifestと製品Brokerへ固定されます。'),
              ),
              const SizedBox(height: 8),
              TextField(
                key: const ValueKey('export-update-public-key'),
                controller: _exportUpdatePublicKeyController,
                minLines: 1,
                maxLines: 2,
                autocorrect: false,
                enableSuggestions: false,
                decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  labelText: 'Ed25519 SPKI公開鍵（DER、小文字hex）',
                ),
              ),
              const SizedBox(height: 8),
              TextField(
                key: const ValueKey('export-update-source'),
                controller: _exportUpdateSourceController,
                autocorrect: false,
                enableSuggestions: false,
                decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  labelText: 'HTTPS配布元base URL',
                  hintText: 'https://updates.example.com/d4-pocket/stable',
                ),
              ),
              const SizedBox(height: 8),
              DropdownButtonFormField<String>(
                key: const ValueKey('export-update-channel'),
                initialValue: _exportUpdateChannel,
                decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  labelText: '配布channel',
                ),
                items: const [
                  DropdownMenuItem(value: 'stable', child: Text('stable')),
                  DropdownMenuItem(value: 'beta', child: Text('beta')),
                  DropdownMenuItem(value: 'nightly', child: Text('nightly')),
                ],
                onChanged: (value) {
                  if (value != null) {
                    setState(() => _exportUpdateChannel = value);
                  }
                },
              ),
            ],
          ),
          Text('任意画面', style: Theme.of(context).textTheme.titleSmall),
          for (final entry in guiShellOptionalExportModules.entries)
            CheckboxListTile(
              key: ValueKey('export-module-${entry.key}'),
              contentPadding: EdgeInsets.zero,
              dense: true,
              value: _selectedExportModules.contains(entry.key),
              title: Text(entry.value),
              subtitle: entry.key == 'shell.trace_inspector'
                  ? const Text('観測Moduleも自動的に含めます')
                  : null,
              onChanged: (selected) => setState(() {
                if (selected == true) {
                  _selectedExportModules.add(entry.key);
                } else {
                  _selectedExportModules.remove(entry.key);
                }
              }),
            ),
          const SizedBox(height: 8),
          FilledButton.icon(
            key: const ValueKey('compose-export-button'),
            onPressed: _exportInProgress ? null : () => _runExport(client),
            icon: _exportInProgress
                ? const SizedBox.square(
                    dimension: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.file_download_outlined),
            label: Text(_exportInProgress
                ? 'Windows Owner確認を待っています'
                : 'Owner確認してManifest fileを生成'),
          ),
          if (_exportMessage != null) ...[
            const SizedBox(height: 8),
            Text(_exportMessage!),
          ],
          if (_exportReceipt != null) ...[
            const SizedBox(height: 8),
            SelectableText(_exportReceipt!),
          ],
        ],
      ),
    );
  }

  Future<void> _runExport(ExportClient client) async {
    final updatePublicKeyDerHex = _exportUpdatePublicKeyController.text.trim();
    final updatePackageSourceUrl = _exportUpdateSourceController.text.trim();
    if (updatePublicKeyDerHex.isNotEmpty != updatePackageSourceUrl.isNotEmpty) {
      _setExportMessage('更新trustを設定する場合は、公開鍵とHTTPS配布元の両方を入力してください。');
      return;
    }
    setState(() {
      _exportInProgress = true;
      _exportMessage = 'Rust起動器の確認画面で許可またはキャンセルしてください。';
    });
    try {
      final receipt = await client.export(
        exportId: 'settings-windows-export',
        composeManifest: _composeManifest(),
        optionalModuleIds: _selectedExportModules.toList()..sort(),
        updatePublicKeyDerHex: updatePublicKeyDerHex,
        updatePackageSourceUrl: updatePackageSourceUrl,
        updatePackageSourceChannel: _exportUpdateChannel,
      );
      _exportReceipt = exportJson(receipt);
      final manifestFile = receipt['manifest_file'];
      final manifestPath = manifestFile is Map ? manifestFile['path'] : null;
      final cleanupPending = manifestFile is Map &&
          manifestFile['temporary_file_status'] == 'cleanup_pending';
      _setExportMessage(cleanupPending
          ? 'Manifest fileは保存されました: ${manifestPath ?? 'Receiptを確認してください'}。一時fileが残る可能性があります。配布せず、ReceiptのRecoveryActionに従ってください。実行可能App生成、binary除去、build、Installer、署名は未実行です。'
          : '独立Manifest fileを保存しました: ${manifestPath ?? '保存先はReceiptを確認してください'}。実行可能App生成、binary除去、build、Installer、署名は未実行です。');
    } catch (error) {
      _setExportMessage('GUI Shell Windows書出しは受理されませんでした: $error');
    } finally {
      if (mounted) setState(() => _exportInProgress = false);
    }
  }

  void _setExportMessage(String message) {
    if (mounted) setState(() => _exportMessage = message);
  }

  Widget _aiEditPanel() {
    final client = _aiEditClient;
    if (client == null) {
      return const BorderedPanel(
        child: Text('GUI Shell編集提案: Broker接続がないため操作できません。自動編集は行いません。'),
      );
    }
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('GUI Shell編集提案', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 4),
          const Text(
              'Owner／Developerが明示開始した提案だけを審査待ちで記録します。Repositoryの自動編集、自己承認、Permission変更は行いません。'),
          const SizedBox(height: 8),
          TextField(
            controller: _editInstructionController,
            maxLines: 2,
            decoration: const InputDecoration(
              border: OutlineInputBorder(),
              labelText: '編集指示',
            ),
          ),
          const SizedBox(height: 8),
          TextField(
            controller: _editTargetPathController,
            decoration: const InputDecoration(
              border: OutlineInputBorder(),
              labelText: '対象path（カンマ区切り）',
            ),
          ),
          const SizedBox(height: 8),
          FilledButton.icon(
            key: const ValueKey('compose-ai-edit-button'),
            onPressed: () => _runAiEdit(client),
            icon: const Icon(Icons.rate_review_outlined),
            label: const Text('編集提案を送信'),
          ),
          if (_aiEditMessage != null) ...[
            const SizedBox(height: 8),
            Text(_aiEditMessage!),
          ],
          if (_aiEditReceipt != null) ...[
            const SizedBox(height: 8),
            SelectableText(_aiEditReceipt!),
          ],
        ],
      ),
    );
  }

  Future<void> _runAiEdit(AiEditClient client) async {
    try {
      final targetPaths = _editTargetPathController.text
          .split(',')
          .map((path) => path.trim())
          .where((path) => path.isNotEmpty)
          .toList(growable: false);
      final receipt = await client.propose(
        editId: 'settings-edit-proposal',
        scope: 'composition',
        instruction: _editInstructionController.text.trim(),
        targetPaths: targetPaths,
        expectedChanges: const ['Owner／Developerが内容を確認してから実装を開始する'],
      );
      _aiEditReceipt = aiEditJson(receipt);
      _setAiEditMessage('編集提案を審査待ちで記録しました。file変更と自動applyは行っていません。');
    } catch (error) {
      _setAiEditMessage('GUI Shell編集提案に失敗しました: $error');
    }
  }

  void _setAiEditMessage(String message) {
    if (mounted) setState(() => _aiEditMessage = message);
  }

  void _setComposeMessage(String message) {
    if (mounted) setState(() => _composeMessage = message);
  }

  Widget _profilePanel() {
    final client = _profileClient;
    if (client == null) {
      return const BorderedPanel(
        child: Text('運用プロファイル: Broker接続がないため操作できません。ローカルsnapshotは権限源ではありません。'),
      );
    }
    return BorderedPanel(
      child: FutureBuilder<List<Map<String, Object?>>>(
        future: _profilesFuture,
        builder: (context, snapshot) {
          final profiles = snapshot.data ?? const <Map<String, Object?>>[];
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('運用プロファイル', style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 4),
              const Text(
                  '再利用可能な要求設定です。適用してもPermission、Approval、Authorityは生成されません。'),
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  SizedBox(
                    width: 180,
                    child: TextField(
                      controller: _profileIdController,
                      decoration: const InputDecoration(labelText: 'プロファイル識別子'),
                    ),
                  ),
                  SizedBox(
                    width: 220,
                    child: TextField(
                      controller: _profileNameController,
                      decoration: const InputDecoration(labelText: '表示名'),
                    ),
                  ),
                  FilledButton.icon(
                    onPressed: _runProfileCreate,
                    icon: const Icon(Icons.add),
                    label: const Text('作成'),
                  ),
                  OutlinedButton.icon(
                    onPressed: () => _runProfileImport(client),
                    icon: const Icon(Icons.file_download_outlined),
                    label: const Text('import'),
                  ),
                  OutlinedButton.icon(
                    onPressed: _refreshProfiles,
                    icon: const Icon(Icons.refresh),
                    label: const Text('更新'),
                  ),
                ],
              ),
              if (_profileMessage != null) ...[
                const SizedBox(height: 8),
                Text(_profileMessage!),
              ],
              if (_exportedProfile != null) ...[
                const SizedBox(height: 8),
                SelectableText(_exportedProfile!),
              ],
              const SizedBox(height: 8),
              if (snapshot.connectionState == ConnectionState.waiting)
                const LinearProgressIndicator()
              else if (snapshot.hasError)
                Text('Profile一覧を取得できません: ${snapshot.error}')
              else if (profiles.isEmpty)
                const Text('保存済みProfileはありません。')
              else
                for (final profile in profiles) _profileRow(client, profile),
            ],
          );
        },
      ),
    );
  }

  Widget _updatePanel() {
    final client = _updateClient;
    if (client == null) {
      return const BorderedPanel(
        child: Text('更新センター: Broker接続がないため操作できません。snapshotは更新権限の根拠ではありません。'),
      );
    }
    return BorderedPanel(
      child: FutureBuilder<Map<String, Object?>>(
        future: _updatesFuture,
        builder: (context, snapshot) {
          final body = snapshot.data ?? const <String, Object?>{};
          final updates = body['更新一覧'] is List
              ? (body['更新一覧']! as List)
                  .whereType<Map>()
                  .map((value) => Map<String, Object?>.from(value))
                  .toList()
              : const <Map<String, Object?>>[];
          final downloadJob = body['download_job'] is Map
              ? Map<String, Object?>.from(body['download_job']! as Map)
              : null;
          final downloadAvailable = body['download実行'] == 'available';
          final rollbackState = body['rollback状態'] is Map
              ? Map<String, Object?>.from(body['rollback状態']! as Map)
              : null;
          final installedProduct = rollbackState?['現在版'] is Map;
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('更新センター', style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 4),
              Text(
                '信頼設定=${body['署名信頼設定'] ?? 'unknown'}。downloadは直接HTTPS接続のみ（system proxy・自動retryなし）で、Rust Desktopの確認が必要です。download完了後に未起動版へ展開し、初回は「インストール」、導入済みなら「更新を有効化」します。別のRust Desktop確認後にpackageとstage全体を再検証し、有効版recordと固定root Bootstrapperを指すStart Menu shortcutを登録します。portable起動元から初回Installした場合は画面を正常終了して導入済み版へ切り替え、導入済み版からの更新は次回起動時に反映します。切替成功後は検証済みdownload packageだけをBroker cacheから除去し、stageと旧versionは保持します。rollbackはBroker記録の直前版へ有効版recordだけを戻します。',
              ),
              if (downloadJob != null) ...[
                const SizedBox(height: 4),
                Text('取得状態: ${UpdateClient.downloadJobLabel(downloadJob)}'),
              ],
              const SizedBox(height: 8),
              OutlinedButton.icon(
                onPressed: _checkingUpdateCatalog || _updateOperationInProgress
                    ? null
                    : _fetchUpdateCatalog,
                icon: const Icon(Icons.refresh),
                label: Text(
                  _checkingUpdateCatalog ? '候補取得中' : '配布元から候補取得',
                ),
              ),
              if (installedProduct) ...[
                const SizedBox(height: 8),
                OutlinedButton.icon(
                  onPressed: _updateOperationInProgress
                      ? null
                      : () => _requestProductRepair(client),
                  icon: const Icon(Icons.build_circle_outlined),
                  label: const Text('起動項目を修復'),
                ),
                const Text(
                  '現行版が検証できる場合に、欠損した固定root起動器とStart Menu shortcutだけを復元します。破損した版本体や有効版記録は上書きしません。',
                ),
                const SizedBox(height: 8),
                OutlinedButton.icon(
                  onPressed: _updateOperationInProgress
                      ? null
                      : () => _requestProductUninstall(client),
                  icon: const Icon(Icons.delete_outline),
                  label: const Text('D4 Pocketをアンインストール'),
                ),
                const Text(
                  '製品fileと一致するStart Menu shortcutを削除します。Credential・設定・Workspace・Audit Storeは保持します。',
                ),
              ],
              if (_updateMessage != null) ...[
                const SizedBox(height: 8),
                Text(_updateMessage!),
              ],
              if (snapshot.connectionState == ConnectionState.waiting)
                const LinearProgressIndicator()
              else if (snapshot.hasError)
                Text('更新一覧を取得できません: ${snapshot.error}')
              else if (updates.isEmpty)
                const Text('署名検査済みの更新候補はありません。')
              else
                for (final update in updates)
                  _updateRow(
                    client,
                    update,
                    downloadAvailable: downloadAvailable,
                    downloadJob: downloadJob,
                    rollbackState: rollbackState,
                  ),
            ],
          );
        },
      ),
    );
  }

  Widget _updateRow(
    UpdateClient client,
    Map<String, Object?> update, {
    required bool downloadAvailable,
    required Map<String, Object?>? downloadJob,
    required Map<String, Object?>? rollbackState,
  }) {
    final updateId = update['更新ID']?.toString() ?? '';
    final candidateHash = update['候補hash']?.toString() ?? '';
    final signatureStatus = UpdateClient.signatureStatusLabel(update['署名状態']);
    final packageSha256 = update['package_sha256']?.toString();
    final packageHashSummary = packageSha256 != null &&
            RegExp(r'^[a-f0-9]{64}$').hasMatch(packageSha256)
        ? 'sha256:${packageSha256.substring(0, 12)}…'
        : 'package未結合';
    final packageSize = update['package_size_bytes'];
    final packageSizeSummary = packageSize is num && packageSize > 0
        ? '$packageSize bytes'
        : 'byte長不明';
    final jobState = downloadJob?['状態'];
    final downloadedForUpdate = UpdateClient.matchesDownloadedCandidate(
      downloadJob,
      updateId: updateId,
      candidateHash: candidateHash,
      packageSha256: packageSha256 ?? '',
    );
    final downloadBusy =
        jobState == 'downloading' || jobState == 'audit_failed';
    final sourceConfigured =
        update['取得元'] is Map && (update['取得元']! as Map)['状態'] == 'configured';
    final canDownload = downloadAvailable &&
        update['署名状態'] == 'verified' &&
        sourceConfigured &&
        !_updateOperationInProgress &&
        !downloadBusy &&
        !downloadedForUpdate;
    final activeVersion = rollbackState?['現在版'];
    final rollbackVersion = rollbackState?['対象版'];
    final isAlreadyActive = activeVersion is Map &&
        activeVersion['更新ID'] == updateId &&
        activeVersion['候補hash'] == candidateHash;
    final isRollbackTarget = rollbackVersion is Map &&
        rollbackVersion['更新ID'] == updateId &&
        rollbackVersion['候補hash'] == candidateHash;
    final canRequestActivation = UpdateClient.canRequestActivation(
      rollbackState: rollbackState,
      staged: _stagedUpdateCandidates.contains('$updateId|$candidateHash'),
      alreadyActive: isAlreadyActive,
      rollbackTarget: isRollbackTarget,
      signatureStatus: update['署名状態'],
      packageSha256: packageSha256,
      packageSize: packageSize,
    );
    final canApply = downloadedForUpdate &&
        !_updateOperationInProgress &&
        !isAlreadyActive &&
        !isRollbackTarget;
    final canRepair = UpdateClient.canRepairActiveVersion(
      alreadyActive: isAlreadyActive,
      downloaded: downloadedForUpdate,
      operationInProgress: _updateOperationInProgress,
      signatureStatus: update['署名状態'],
      packageSha256: packageSha256,
      packageSize: packageSize,
    );
    final rollbackCurrent = rollbackState?['現在版'];
    final canRollback = rollbackState?['状態'] == 'available' &&
        !_updateOperationInProgress &&
        rollbackCurrent is Map &&
        rollbackCurrent['更新ID'] == updateId &&
        rollbackCurrent['候補hash'] == candidateHash &&
        update['署名状態'] == 'verified' &&
        update['rollback可能'] == true;
    final activationLabel = UpdateClient.activationActionLabel(
      rollbackState,
      alreadyActive: isAlreadyActive,
    );
    final activationSuccess = UpdateClient.isFirstInstall(rollbackState)
        ? 'D4 Pocketを固定導入先へインストールし、Start Menuへ登録しました。'
        : '有効版を切り替え、Start Menuへ登録しました。選択版は次回起動時に使用します。';
    return ListTile(
      contentPadding: EdgeInsets.zero,
      title: Text('${update['提供版'] ?? ''} ($updateId)'),
      subtitle: Text(
        'channel=${update['channel']} / 署名=$signatureStatus / rollback=${update['rollback可能']}\n'
        '$packageSizeSummary / $packageHashSummary / ${UpdateClient.packageSourceLabel(update['取得元'])}',
        maxLines: 3,
        overflow: TextOverflow.ellipsis,
      ),
      trailing: Wrap(
        spacing: 4,
        children: [
          TextButton(
            onPressed: canDownload
                ? () => _runUpdateRequest(
                      () => client.requestDownload(
                        updateId: updateId,
                        candidateHash: candidateHash,
                      ),
                      'download jobを開始しました。取得状態は一覧を更新して確認できます。',
                    )
                : null,
            child: Text(downloadedForUpdate ? '取得済み' : 'download'),
          ),
          TextButton(
            onPressed: canApply || canRepair
                ? () => _runUpdateRequest(
                      () async {
                        final body = await client.requestApply(
                          updateId: updateId,
                          candidateHash: candidateHash,
                        );
                        if (body['導入状態'] == 'version_staged') {
                          setState(() => _stagedUpdateCandidates
                              .add('$updateId|$candidateHash'));
                        }
                        return body;
                      },
                      isAlreadyActive
                          ? '現在の有効版を署名済みpackageと照合しました。不足fileだけを復元し、不一致fileは上書きせず拒否します。有効版recordとprocessは変更していません。'
                          : '署名済みpackageを未起動versionとして固定導入先へ展開しました。Start Menu切替・process起動・rollbackは別操作です。',
                    )
                : null,
            child: Text(isAlreadyActive ? '有効版を修復' : '未起動版へ展開'),
          ),
          TextButton(
            onPressed: canRequestActivation && !_updateOperationInProgress
                ? () => _runUpdateRequest(
                      () => client.requestActivation(
                        updateId: updateId,
                        candidateHash: candidateHash,
                      ),
                      '$activationSuccess 検証済みdownload package cacheだけを除去し、stageと旧versionは保持しています。processはこの場で起動していません。',
                    )
                : null,
            child: Text(activationLabel),
          ),
          TextButton(
            onPressed: _updateOperationInProgress
                ? null
                : () => _runUpdateRequest(
                      () => client.defer(
                        updateId: updateId,
                        candidateHash: candidateHash,
                        deferredUntil: DateTime.now()
                            .toUtc()
                            .add(const Duration(days: 1))
                            .toIso8601String(),
                      ),
                      '更新を延期しました。',
                    ),
            child: const Text('延期'),
          ),
          if (canRollback)
            TextButton(
              onPressed: () => _runUpdateRequest(
                () => client.requestRollback(
                  updateId: updateId,
                  candidateHash: candidateHash,
                ),
                '直前に有効化した版へ戻しました。有効版切替は次回の固定root起動時に反映されます。process起動・file削除はしていません。',
              ),
              child: const Text('直前版へ戻す'),
            ),
        ],
      ),
    );
  }

  Future<void> _fetchUpdateCatalog() async {
    final client = _updateClient;
    if (client == null || _updateOperationInProgress) return;
    setState(() {
      _checkingUpdateCatalog = true;
      _updateOperationInProgress = true;
    });
    try {
      final result = await client.fetchCandidates();
      final state = result['状態'];
      final count = result['候補件数'];
      _setUpdateMessage(
        state == 'unconfigured'
            ? 'Brokerに署名済み配布元が構成されていません。'
            : 'Brokerが署名検証済み候補を取得しました（${count ?? 0}件）。',
      );
    } catch (error) {
      _setUpdateMessage('更新候補の取得に失敗しました: $error');
    } finally {
      if (mounted) {
        setState(() {
          _checkingUpdateCatalog = false;
          _updateOperationInProgress = false;
          _updatesFuture = client.list();
        });
      }
    }
  }

  void _loadUpdates() {
    final client = _updateClient;
    if (!mounted || client == null) return;
    setState(() {
      _updatesFuture = client.list();
    });
  }

  Future<void> _runUpdateRequest(
    Future<Map<String, Object?>> Function() request,
    String success,
  ) async {
    if (_updateOperationInProgress) return;
    setState(() => _updateOperationInProgress = true);
    var initialInstallAccepted = false;
    var handoffToInstalledVersion = false;
    try {
      final body = await request();
      if (UpdateClient.shouldLaunchInstalledVersionAfterExit(body)) {
        initialInstallAccepted = true;
        handoffToInstalledVersion = await _completeInitialInstallHandoff();
        if (!handoffToInstalledVersion) {
          _setUpdateMessage(
            'インストール済み版は登録されましたが、画面を終了できませんでした。Start Menuから起動してください。',
          );
        }
      } else {
        final state = body['実行状態']?.toString();
        if (body['download package cleanup'] == 'pending') {
          _setUpdateMessage(
            '$success package cache清掃は保留中です。stageと有効版recordは維持しています。',
          );
        } else {
          _setUpdateMessage(
            state == 'suspended' ? '$success Brokerは実行を保留しました。' : success,
          );
        }
      }
    } catch (error) {
      _setUpdateMessage(initialInstallAccepted
          ? 'インストール済み版は登録されましたが、切替のための画面終了に失敗しました。Start Menuから起動してください。'
          : '更新操作失敗: $error');
    } finally {
      if (mounted && !handoffToInstalledVersion) {
        setState(() => _updateOperationInProgress = false);
      }
    }
    if (!handoffToInstalledVersion) _loadUpdates();
  }

  Future<bool> _completeInitialInstallHandoff() async {
    _setUpdateMessage('インストールが完了しました。導入済み版を起動します。');
    await Future<void>.delayed(const Duration(milliseconds: 600));
    final response = await (widget.requestApplicationExit?.call(
          ui.AppExitType.required,
        ) ??
        ServicesBinding.instance.exitApplication(ui.AppExitType.required));
    return response == ui.AppExitResponse.exit;
  }

  Future<void> _requestProductUninstall(UpdateClient client) async {
    if (_updateOperationInProgress) return;
    setState(() => _updateOperationInProgress = true);
    var uninstallAuthorized = false;
    var applicationExitAccepted = false;
    try {
      final body = await client.requestUninstall();
      if (body['状態'] != 'uninstall_authorized') {
        throw const BrokerClientException('アンインストール許可を確認できません');
      }
      uninstallAuthorized = true;
      _setUpdateMessage(
          'Rust Brokerが削除意図をAuditへ記録しました。画面を終了して固定導入先を削除します。利用者データは保持されます。');
      await Future<void>.delayed(const Duration(milliseconds: 500));
      final response = await (widget.requestApplicationExit?.call(
            ui.AppExitType.required,
          ) ??
          ServicesBinding.instance.exitApplication(ui.AppExitType.required));
      applicationExitAccepted = response == ui.AppExitResponse.exit;
      if (!applicationExitAccepted) {
        _setUpdateMessage(
          'アプリの終了がキャンセルされました。製品fileはまだ削除されていません。再度アンインストールを要求してください。',
        );
      }
    } catch (error) {
      _setUpdateMessage(uninstallAuthorized
          ? '削除許可は記録されましたが、アプリを終了できませんでした。製品fileはまだ削除されていません。再度アンインストールを要求してください。'
          : 'アンインストール要求に失敗しました: $error');
    } finally {
      if (mounted && !applicationExitAccepted) {
        setState(() => _updateOperationInProgress = false);
      }
    }
    if (!applicationExitAccepted) _loadUpdates();
  }

  Future<void> _requestProductRepair(UpdateClient client) async {
    if (_updateOperationInProgress) return;
    setState(() => _updateOperationInProgress = true);
    try {
      final body = await client.requestProductRepair();
      final state = body['状態'];
      if (state != 'product_launch_entries_repaired' && state != 'unchanged') {
        throw const BrokerClientException('起動項目修復の完了状態を確認できません');
      }
      final restored = <String>[
        if (body['起動器復元'] == true) '固定root起動器',
        if (body['Start Menu復元'] == true) 'Start Menu shortcut',
      ];
      _setUpdateMessage(restored.isEmpty
          ? '固定root起動器とStart Menu shortcutは既に有効です。変更はありません。'
          : '${restored.join('、')}を復元しました。版本体と利用者dataは変更していません。');
    } catch (error) {
      _setUpdateMessage('起動項目を修復できませんでした。既存のfileは上書きしていません: $error');
    } finally {
      if (mounted) {
        setState(() => _updateOperationInProgress = false);
      }
    }
    _loadUpdates();
  }

  void _setUpdateMessage(String message) {
    if (!mounted) return;
    setState(() => _updateMessage = message);
  }

  Widget _profileRow(ProfileClient client, Map<String, Object?> profile) {
    final id = profile['ProfileID']?.toString() ?? '';
    final name = profile['表示名']?.toString() ?? id;
    return ListTile(
      contentPadding: EdgeInsets.zero,
      title: Text('$name ($id)'),
      subtitle: Text(
        '実行系=${profile['Runtime']} / Adapter=${profile['Adapter']}',
      ),
      trailing: Wrap(
        spacing: 4,
        children: [
          TextButton(
            onPressed: () => _runProfileApply(client, profile),
            child: const Text('適用要求'),
          ),
          TextButton(
            onPressed: () => _runProfileCopy(client, profile),
            child: const Text('複製'),
          ),
          TextButton(
            onPressed: () => _runProfileExport(client, id),
            child: const Text('export'),
          ),
          TextButton(
            onPressed: () => _runProfileDelete(client, id),
            child: const Text('削除'),
          ),
        ],
      ),
    );
  }

  void _refreshProfiles() {
    final client = _profileClient;
    if (client == null) return;
    setState(() {
      _profilesFuture = client.list();
    });
  }

  Future<void> _runProfileCreate() async {
    final client = _profileClient;
    if (client == null) return;
    try {
      await client.create({
        '版': 1,
        'ProfileID': _profileIdController.text.trim(),
        '表示名': _profileNameController.text.trim(),
        'Runtime': 'runtime.local',
        'Adapter': 'adapter.local',
        '要求Capability': ['filesystem.read'],
        'Content Exposure': {'visibility': 'redacted'},
        'network exposure': 'loopback',
        'resource limit': {
          'cpu_millis': 1000,
          'memory_bytes': 1073741824,
          'max_processes': 2,
          'max_output_bytes': 1048576,
        },
        'UI preference': {
          'theme': 'system',
          'density': 'comfortable',
          'locale': 'ja-JP',
        },
      });
      _setProfileMessage('Profileを作成しました。');
    } catch (error) {
      _setProfileMessage('Profile作成失敗: $error');
    }
    _refreshProfiles();
  }

  Future<void> _runProfileApply(
      ProfileClient client, Map<String, Object?> profile) async {
    try {
      await client.apply(
        profileId: profile['ProfileID']!.toString(),
        profileHash: profile['profile_hash']?.toString() ?? '',
      );
      _setProfileMessage('Profile適用要求を記録しました。権限は変更していません。');
    } catch (error) {
      _setProfileMessage('Profile適用要求失敗: $error');
    }
  }

  Future<void> _runProfileCopy(
    ProfileClient client,
    Map<String, Object?> profile,
  ) async {
    final sourceId = profile['ProfileID']?.toString() ?? '';
    try {
      await client.copy(
        sourceProfileId: sourceId,
        profileId: '$sourceId.copy',
        displayName: '${profile['表示名'] ?? sourceId} の複製',
      );
      _setProfileMessage('Profileを複製しました。');
    } catch (error) {
      _setProfileMessage('Profile複製失敗: $error');
    }
    _refreshProfiles();
  }

  Future<void> _runProfileExport(ProfileClient client, String id) async {
    try {
      final receipt = await client.export(id);
      final profile = receipt['Profile'];
      _exportedProfile = profile is Map
          ? profileJson(Map<String, Object?>.from(profile))
          : null;
      _setProfileMessage('Profile設定をexportしました。');
    } catch (error) {
      _setProfileMessage('Profile export失敗: $error');
    }
  }

  Future<void> _runProfileDelete(ProfileClient client, String id) async {
    try {
      await client.delete(profileId: id);
      _setProfileMessage('Profileを削除しました。');
    } catch (error) {
      _setProfileMessage('Profile削除失敗: $error');
    }
    _refreshProfiles();
  }

  Future<void> _runProfileImport(ProfileClient client) async {
    final controller = TextEditingController();
    final raw = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Profile設定をimport'),
        content: TextField(
          controller: controller,
          minLines: 5,
          maxLines: 12,
          decoration: const InputDecoration(
            border: OutlineInputBorder(),
            hintText: 'exportしたProfile JSONを貼り付けてください',
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(controller.text),
            child: const Text('import'),
          ),
        ],
      ),
    );
    controller.dispose();
    if (raw == null || raw.trim().isEmpty) return;
    try {
      final decoded = jsonDecode(raw);
      if (decoded is! Map) throw const FormatException('objectではありません');
      await client.importProfile(Map<String, Object?>.from(decoded));
      _setProfileMessage('Profileをimportしました。');
    } catch (error) {
      _setProfileMessage('Profile import失敗: $error');
    }
    _refreshProfiles();
  }

  void _setProfileMessage(String message) {
    if (mounted) setState(() => _profileMessage = message);
  }

  bool _matchesFilters(SettingRecord setting) {
    final query = _searchController.text.trim().toLowerCase();
    final searchable = [
      setting.key,
      setting.group,
      setting.defaultValue,
      setting.currentValue,
      setting.effectiveValue,
      setting.source,
      _flags(setting),
    ].join(' ').toLowerCase();
    if (query.isNotEmpty && !searchable.contains(query)) {
      return false;
    }
    if (_modifiedOnly && !setting.modified) {
      return false;
    }
    if (_authorityOnly && !setting.authorityRelated) {
      return false;
    }
    if (_dangerousOnly && !setting.dangerous) {
      return false;
    }
    if (_phaseReleaseOnly &&
        !searchable.contains('phase') &&
        !searchable.contains('release')) {
      return false;
    }
    return true;
  }

  DataRow _settingRow(SettingRecord setting) {
    return DataRow(
      cells: [
        DataCell(Text('${setting.key}\n${setting.group}')),
        DataCell(Text(setting.currentValue)),
        DataCell(Text(setting.source)),
        DataCell(
          Text('既定値: ${setting.defaultValue}\n有効値: ${setting.effectiveValue}'),
        ),
        DataCell(Text(_flags(setting))),
      ],
    );
  }

  String _flags(SettingRecord setting) {
    final flags = [
      if (setting.modified) '変更済み',
      if (setting.dangerous) '危険',
      if (setting.authorityRelated) '権限関連',
    ];
    return flags.isEmpty ? 'なし' : flags.join(', ');
  }
}

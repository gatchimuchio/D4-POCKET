import 'dart:async';
import 'dart:ui' show PlatformDispatcher;

import 'package:flutter/material.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter/services.dart';

import 'models/generated_contracts.dart';
import 'screens/approval_center.dart';
import 'screens/audit_viewer.dart';
import 'screens/dashboard.dart';
import 'screens/evidence_center.dart';
import 'screens/evaluation_lab.dart';
import 'screens/agent_center.dart';
import 'screens/authority_map.dart';
import 'screens/problems_panel.dart';
import 'screens/recovery_center.dart';
import 'screens/runtime_center.dart';
import 'screens/runtime_dialogue.dart';
import 'screens/history_screen.dart';
import 'screens/host_capability_center.dart';
import 'screens/host_operation_center.dart';
import 'screens/notifications.dart';
import 'screens/observability_center.dart';
import 'screens/trace_inspector.dart';
import 'screens/settings.dart';
import 'screens/shared.dart';
import 'screens/setup_doctor.dart';
import 'screens/trust_center.dart';
import 'services/runtime_resource_client.dart';
import 'services/runtime_lifecycle_client.dart';
import 'services/evaluation_client.dart';
import 'services/global_search_index.dart';
import 'services/setup_doctor_export.dart';
import 'services/shell_core_client.dart';
import 'services/surface_semantics_export.dart';
import 'services/windows_tray_client.dart';

const String kD4PocketProductTitle = 'D4 Pocket';
const String kGuiShellProductTitle = 'D4 Pocket powered by GUI Shell';
const double _navigationRailMinScrollableExtent = 960;
SemanticsHandle? _productSemanticsHandle;

Future<void> main() async {
  await runZonedGuarded<Future<void>>(
    () async {
      WidgetsFlutterBinding.ensureInitialized();
      // Windows UI AutomationはSemantics収集を要求するまでFlutter treeを公開しない。
      // 製品起動中はSemanticsHandleを保持し、正式なaccessibility treeへ接続する。
      _productSemanticsHandle ??= SemanticsBinding.instance.ensureSemantics();
      _installFatalErrorHandlers();
      final client = await ShellCoreClient.product();
      await writeSetupDoctorProductExportIfRequested(client.getSnapshot());
      runApp(GuiShellDesktopApp(client: client));
      WidgetsBinding.instance.addPostFrameCallback((_) {
        unawaited(writeSurfaceSemanticsExportIfRequested());
      });
    },
    (error, stack) {
      FlutterError.reportError(
        FlutterErrorDetails(
          exception: error,
          stack: stack,
          library: 'gui_shell_desktop',
          context: ErrorDescription('アプリ領域で捕捉されなかったエラー'),
        ),
      );
    },
  );
}

void _installFatalErrorHandlers() {
  FlutterError.onError = (details) {
    FlutterError.presentError(details);
  };
  PlatformDispatcher.instance.onError = (error, stack) {
    FlutterError.reportError(
      FlutterErrorDetails(
        exception: error,
        stack: stack,
        library: 'gui_shell_desktop',
        context: ErrorDescription('プラットフォーム配送処理で捕捉されなかったエラー'),
      ),
    );
    return true;
  };
  ErrorWidget.builder = (details) =>
      GuiShellFatalErrorScreen(message: details.exceptionAsString());
}

class GuiShellDesktopApp extends StatelessWidget {
  const GuiShellDesktopApp({
    super.key,
    this.client,
    this.runtimeResourceClient,
    this.runtimeResourceConnect,
    this.runtimeLifecycleClient,
    this.runtimeLifecycleConnect,
    this.evaluationClient,
    this.evaluationConnect,
  });

  final ShellCoreClient? client;
  final RuntimeResourceClient? runtimeResourceClient;
  final RuntimeResourceClientConnector? runtimeResourceConnect;
  final RuntimeLifecycleClient? runtimeLifecycleClient;
  final RuntimeLifecycleClientConnector? runtimeLifecycleConnect;
  final EvaluationClient? evaluationClient;
  final EvaluationClientConnector? evaluationConnect;

  @override
  Widget build(BuildContext context) {
    final client = this.client ?? ShellCoreClient.mock();
    return MaterialApp(
      title: kGuiShellProductTitle,
      debugShowCheckedModeBanner: false,
      themeMode: ThemeMode.system,
      theme: _buildShellTheme(Brightness.light),
      darkTheme: _buildShellTheme(Brightness.dark),
      home: ShellHomePage(
        client: client,
        runtimeResourceClient: runtimeResourceClient,
        runtimeResourceConnect: runtimeResourceConnect,
        runtimeLifecycleClient: runtimeLifecycleClient,
        runtimeLifecycleConnect: runtimeLifecycleConnect,
        evaluationClient: evaluationClient,
        evaluationConnect: evaluationConnect,
      ),
    );
  }
}

ThemeData _buildShellTheme(Brightness brightness) {
  final scheme = ColorScheme.fromSeed(
    seedColor: const Color(0xff2f6f5e),
    brightness: brightness,
  );
  return ThemeData(
    colorScheme: scheme,
    useMaterial3: true,
    visualDensity: VisualDensity.compact,
    scaffoldBackgroundColor: scheme.surface,
  );
}

class GuiShellFatalErrorScreen extends StatelessWidget {
  const GuiShellFatalErrorScreen({super.key, required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    final scheme = ColorScheme.fromSeed(
      seedColor: const Color(0xff2f6f5e),
      brightness: Brightness.light,
    );
    return Directionality(
      textDirection: TextDirection.ltr,
      child: Material(
        color: scheme.errorContainer,
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 720),
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Icon(Icons.error_outline, color: scheme.onErrorContainer),
                  const SizedBox(height: 12),
                  Text(
                    'GUI Shellで回復不能な画面エラーが発生しました。',
                    style: TextStyle(
                      color: scheme.onErrorContainer,
                      fontSize: 18,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: 8),
                  Text(
                    message,
                    style: TextStyle(color: scheme.onErrorContainer),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class ShellHomePage extends StatefulWidget {
  const ShellHomePage({
    super.key,
    required this.client,
    this.runtimeResourceClient,
    this.runtimeResourceConnect,
    this.runtimeLifecycleClient,
    this.runtimeLifecycleConnect,
    this.evaluationClient,
    this.evaluationConnect,
  });

  final ShellCoreClient client;
  final RuntimeResourceClient? runtimeResourceClient;
  final RuntimeResourceClientConnector? runtimeResourceConnect;
  final RuntimeLifecycleClient? runtimeLifecycleClient;
  final RuntimeLifecycleClientConnector? runtimeLifecycleConnect;
  final EvaluationClient? evaluationClient;
  final EvaluationClientConnector? evaluationConnect;

  @override
  State<ShellHomePage> createState() => _ShellHomePageState();
}

class _ShellHomePageState extends State<ShellHomePage> {
  int selectedIndex = 0;
  bool _dialogueVisited = false;
  _ShellViewMode viewMode = _ShellViewMode.ownerUse;
  _ShellNavigationGroup navigationGroup = _ShellNavigationGroup.all;
  late final WindowsTrayClient _trayClient;
  Timer? _trayRefreshTimer;

  @override
  void initState() {
    super.initState();
    _trayClient = WindowsTrayClient();
    unawaited(_initializeWindowsTray());
  }

  @override
  void dispose() {
    _trayRefreshTimer?.cancel();
    unawaited(_trayClient.dispose());
    super.dispose();
  }

  Future<void> _initializeWindowsTray() async {
    await _trayClient.start(onAction: _handleWindowsTrayAction);
    if (!mounted) return;
    await _publishWindowsTrayProjection();
    _trayRefreshTimer = Timer.periodic(const Duration(seconds: 30), (_) {
      unawaited(_publishWindowsTrayProjection());
    });
  }

  Future<void> _publishWindowsTrayProjection() async {
    final projection = await WindowsTrayProjection.fromSnapshot(
      widget.client.getSnapshot(),
      widget.client.brokerTransport,
    );
    if (mounted) {
      await _trayClient.publish(projection);
    }
  }

  Future<void> _handleWindowsTrayAction(String action) async {
    if (action == 'open') {
      if (mounted) _selectPage(0);
      return;
    }
    if (action != 'stop_request') return;
    try {
      final result = await widget.client.requestAllRuntimeStop();
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(result.message)),
        );
      }
    } on Object catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('全Runtime停止要求を送信できません: $error')),
        );
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final snapshot = widget.client.getSnapshot();
    _dialogueVisited = _dialogueVisited || selectedIndex == 12;
    final pages = [
      Dashboard(client: widget.client),
      SetupDoctor(client: widget.client),
      TrustCenter(client: widget.client),
      RuntimeCenter(
        client: widget.client,
        resourceClient: widget.runtimeResourceClient,
        connectResource:
            widget.runtimeResourceConnect ?? connectRuntimeResourceClient,
        lifecycleClient: widget.runtimeLifecycleClient,
        connectLifecycle:
            widget.runtimeLifecycleConnect ?? connectRuntimeLifecycleClient,
      ),
      AuthorityMap(client: widget.client),
      AgentCenter(client: widget.client),
      ApprovalCenter(client: widget.client),
      AuditViewer(client: widget.client),
      RecoveryCenter(client: widget.client),
      ProblemsPanel(client: widget.client),
      EvidenceCenter(client: widget.client),
      SettingsScreen(client: widget.client),
      if (_dialogueVisited)
        RuntimeDialogueScreen(readOnly: viewMode == _ShellViewMode.demo)
      else
        const SizedBox.shrink(),
      if (selectedIndex == 13 && viewMode == _ShellViewMode.ownerUse)
        const HistoryScreen()
      else
        const SizedBox.shrink(),
      EvaluationLab(
        client: widget.evaluationClient,
        connect: widget.evaluationConnect ?? connectEvaluationClient,
      ),
      HostCapabilityCenter(client: widget.client),
      NotificationsScreen(
        client: widget.client,
        onNavigate: _navigateFromNotification,
      ),
      ObservabilityCenter(client: widget.client),
      TraceInspector(client: widget.client),
      HostOperationCenter(client: widget.client),
    ];
    final pageEntries = _pageEntries();
    final visiblePageEntries = pageEntries
        .where((page) => navigationGroup.includes(page.group))
        .toList(growable: false);

    return Shortcuts(
      shortcuts: const {
        SingleActivator(LogicalKeyboardKey.keyK, control: true):
            _OpenCommandPaletteIntent(),
        SingleActivator(LogicalKeyboardKey.keyP, control: true):
            _OpenCommandPaletteIntent(),
        SingleActivator(
          LogicalKeyboardKey.keyF,
          control: true,
          shift: true,
        ): _OpenGlobalSearchIntent(),
      },
      child: Actions(
        actions: {
          _OpenCommandPaletteIntent: CallbackAction<_OpenCommandPaletteIntent>(
            onInvoke: (_) {
              _openCommandPalette(context, snapshot, pageEntries);
              return null;
            },
          ),
          _OpenGlobalSearchIntent: CallbackAction<_OpenGlobalSearchIntent>(
            onInvoke: (_) {
              _openGlobalSearch(context, snapshot);
              return null;
            },
          ),
        },
        child: Focus(
          autofocus: true,
          child: Scaffold(
            body: Column(
              children: [
                _TopCommandBar(
                  selectedLabel: pageEntries[selectedIndex].label,
                  viewMode: viewMode,
                  navigationGroup: navigationGroup,
                  onViewModeChanged: (mode) => setState(() => viewMode = mode),
                  onNavigationGroupChanged: _setNavigationGroup,
                  onOpenCommandPalette: () =>
                      _openCommandPalette(context, snapshot, pageEntries),
                  onOpenGlobalSearch: () =>
                      _openGlobalSearch(context, snapshot),
                ),
                PhaseBanner(snapshot: snapshot),
                Expanded(
                  child: Row(
                    children: [
                      SurfaceSemantics(
                        label: 'ナビゲーション',
                        evidenceLabel: 'NavigationRail',
                        explicitChildNodes: true,
                        child: LayoutBuilder(
                          builder: (context, constraints) {
                            final railHeight = constraints.hasBoundedHeight &&
                                    constraints.maxHeight >
                                        _navigationRailMinScrollableExtent
                                ? constraints.maxHeight
                                : _navigationRailMinScrollableExtent;
                            return SingleChildScrollView(
                              child: SizedBox(
                                height: railHeight,
                                child: NavigationRail(
                                  selectedIndex: visiblePageEntries.indexWhere(
                                    (page) => page.index == selectedIndex,
                                  ),
                                  onDestinationSelected: (index) => _selectPage(
                                      visiblePageEntries[index].index),
                                  labelType: NavigationRailLabelType.selected,
                                  destinations: navigationGroup ==
                                          _ShellNavigationGroup.all
                                      ? const [
                                          NavigationRailDestination(
                                            icon:
                                                Icon(Icons.dashboard_outlined),
                                            selectedIcon: Icon(Icons.dashboard),
                                            label: Text('概要'),
                                          ),
                                          NavigationRailDestination(
                                            icon: Icon(
                                                Icons.build_circle_outlined),
                                            selectedIcon:
                                                Icon(Icons.build_circle),
                                            label: Text('診断'),
                                          ),
                                          NavigationRailDestination(
                                            icon: Icon(
                                                Icons.verified_user_outlined),
                                            selectedIcon:
                                                Icon(Icons.verified_user),
                                            label: Text('信頼'),
                                          ),
                                          NavigationRailDestination(
                                            icon: Icon(Icons.hub_outlined),
                                            selectedIcon: Icon(Icons.hub),
                                            label: Text('実行系'),
                                          ),
                                          NavigationRailDestination(
                                            icon: Icon(
                                                Icons.account_tree_outlined),
                                            selectedIcon:
                                                Icon(Icons.account_tree),
                                            label: Text('権限'),
                                          ),
                                          NavigationRailDestination(
                                            icon:
                                                Icon(Icons.smart_toy_outlined),
                                            selectedIcon: Icon(Icons.smart_toy),
                                            label: Text('エージェント'),
                                          ),
                                          NavigationRailDestination(
                                            icon:
                                                Icon(Icons.fact_check_outlined),
                                            selectedIcon:
                                                Icon(Icons.fact_check),
                                            label: Text('承認'),
                                          ),
                                          NavigationRailDestination(
                                            icon: Icon(
                                                Icons.receipt_long_outlined),
                                            selectedIcon:
                                                Icon(Icons.receipt_long),
                                            label: Text('監査'),
                                          ),
                                          NavigationRailDestination(
                                            icon: Icon(
                                              Icons.health_and_safety_outlined,
                                            ),
                                            selectedIcon: Icon(
                                              Icons.health_and_safety,
                                            ),
                                            label: Text('復旧'),
                                          ),
                                          NavigationRailDestination(
                                            icon: Icon(
                                                Icons.report_problem_outlined),
                                            selectedIcon:
                                                Icon(Icons.report_problem),
                                            label: Text('問題'),
                                          ),
                                          NavigationRailDestination(
                                            icon: Icon(
                                                Icons.inventory_2_outlined),
                                            selectedIcon:
                                                Icon(Icons.inventory_2),
                                            label: Text('証拠'),
                                          ),
                                          NavigationRailDestination(
                                            icon: Icon(Icons.settings_outlined),
                                            selectedIcon: Icon(Icons.settings),
                                            label: Text('設定'),
                                          ),
                                          NavigationRailDestination(
                                            icon:
                                                Icon(Icons.chat_bubble_outline),
                                            selectedIcon:
                                                Icon(Icons.chat_bubble),
                                            label: Text('対話'),
                                          ),
                                          NavigationRailDestination(
                                            icon: Icon(Icons.history_outlined),
                                            selectedIcon: Icon(Icons.history),
                                            label: Text('履歴'),
                                          ),
                                          NavigationRailDestination(
                                            icon: Icon(Icons.science_outlined),
                                            selectedIcon: Icon(Icons.science),
                                            label: Text('評価ラボ'),
                                          ),
                                          NavigationRailDestination(
                                            icon: Icon(Icons.public_outlined),
                                            selectedIcon: Icon(Icons.public),
                                            label: Text('ホスト能力'),
                                          ),
                                          NavigationRailDestination(
                                            icon:
                                                Icon(Icons.notifications_none),
                                            selectedIcon:
                                                Icon(Icons.notifications),
                                            label: Text('通知'),
                                          ),
                                          NavigationRailDestination(
                                            icon: Icon(Icons.insights_outlined),
                                            selectedIcon: Icon(Icons.insights),
                                            label: Text('観測'),
                                          ),
                                          NavigationRailDestination(
                                            icon: Icon(Icons.timeline_outlined),
                                            selectedIcon: Icon(Icons.timeline),
                                            label: Text('追跡'),
                                          ),
                                          NavigationRailDestination(
                                            icon:
                                                Icon(Icons.swap_horiz_outlined),
                                            selectedIcon:
                                                Icon(Icons.swap_horiz),
                                            label: Text('Host操作'),
                                          ),
                                        ]
                                      : [
                                          for (final page in visiblePageEntries)
                                            NavigationRailDestination(
                                              icon: Icon(page.icon),
                                              selectedIcon: Icon(page.icon),
                                              label: Text(page.label),
                                            ),
                                        ],
                                ),
                              ),
                            );
                          },
                        ),
                      ),
                      const VerticalDivider(width: 1),
                      Expanded(
                          child: IndexedStack(
                              index: selectedIndex, children: pages)),
                    ],
                  ),
                ),
                ShellStatusBar(
                    snapshot: snapshot, dialogueContext: selectedIndex == 12),
              ],
            ),
          ),
        ),
      ),
    );
  }

  List<_ShellPageEntry> _pageEntries() {
    return const [
      _ShellPageEntry(
          0, '概要', Icons.dashboard_outlined, _ShellNavigationGroup.operation),
      _ShellPageEntry(1, '環境診断', Icons.build_circle_outlined,
          _ShellNavigationGroup.development),
      _ShellPageEntry(2, '信頼センター', Icons.verified_user_outlined,
          _ShellNavigationGroup.safety),
      _ShellPageEntry(
          3, '実行系センター', Icons.hub_outlined, _ShellNavigationGroup.operation),
      _ShellPageEntry(4, '権限対応図', Icons.account_tree_outlined,
          _ShellNavigationGroup.safety),
      _ShellPageEntry(5, 'エージェントセンター', Icons.smart_toy_outlined,
          _ShellNavigationGroup.operation),
      _ShellPageEntry(
          6, '承認センター', Icons.fact_check_outlined, _ShellNavigationGroup.safety),
      _ShellPageEntry(7, '監査ビューアー', Icons.receipt_long_outlined,
          _ShellNavigationGroup.safety),
      _ShellPageEntry(8, '復旧手順', Icons.health_and_safety_outlined,
          _ShellNavigationGroup.safety),
      _ShellPageEntry(9, '問題一覧', Icons.report_problem_outlined,
          _ShellNavigationGroup.safety),
      _ShellPageEntry(10, '証拠センター', Icons.inventory_2_outlined,
          _ShellNavigationGroup.safety),
      _ShellPageEntry(
          11, '設定', Icons.settings_outlined, _ShellNavigationGroup.settings),
      _ShellPageEntry(12, '実行系との対話', Icons.chat_bubble_outline,
          _ShellNavigationGroup.operation),
      _ShellPageEntry(
          13, '実行履歴', Icons.history, _ShellNavigationGroup.operation),
      _ShellPageEntry(
          14, '評価ラボ', Icons.science_outlined, _ShellNavigationGroup.operation),
      _ShellPageEntry(
          15, 'ホスト能力', Icons.public_outlined, _ShellNavigationGroup.operation),
      _ShellPageEntry(16, '通知センター', Icons.notifications_none,
          _ShellNavigationGroup.operation),
      _ShellPageEntry(17, '観測センター', Icons.insights_outlined,
          _ShellNavigationGroup.operation),
      _ShellPageEntry(18, '追跡情報', Icons.timeline_outlined,
          _ShellNavigationGroup.development),
      _ShellPageEntry(19, 'Host操作面', Icons.swap_horiz_outlined,
          _ShellNavigationGroup.operation),
    ];
  }

  void _navigateFromNotification(String target) {
    final index = switch (target) {
      'approval' => 6,
      'runtime' => 3,
      'audit' => 7,
      'recovery' => 8,
      'device' => 3,
      'agent' => 5,
      'settings' => 11,
      'evaluation' => 14,
      _ => 0,
    };
    if (!mounted) return;
    _selectPage(index);
  }

  void _setNavigationGroup(_ShellNavigationGroup group) {
    final pageEntries = _pageEntries()
        .where((page) => group.includes(page.group))
        .toList(growable: false);
    if (pageEntries.isEmpty) return;
    final currentPageIsVisible = pageEntries.any(
      (page) => page.index == selectedIndex,
    );
    setState(() {
      navigationGroup = group;
      if (!currentPageIsVisible) {
        selectedIndex = pageEntries.first.index;
      }
    });
  }

  void _selectPage(int index) {
    final page = _pageEntries().firstWhere((entry) => entry.index == index);
    setState(() {
      selectedIndex = index;
      if (!navigationGroup.includes(page.group)) {
        navigationGroup = page.group;
      }
    });
  }

  Future<void> _openCommandPalette(
    BuildContext context,
    ShellSnapshot snapshot,
    List<_ShellPageEntry> pageEntries,
  ) async {
    final selected = await showDialog<_CommandEntry>(
      context: context,
      builder: (context) => _CommandPaletteDialog(
        entries: _commandEntries(snapshot, pageEntries),
      ),
    );
    if (selected == null || !mounted) {
      return;
    }
    if (selected.copyText != null) {
      await Clipboard.setData(ClipboardData(text: selected.copyText!));
      if (!mounted) {
        return;
      }
    }
    _selectPage(selected.pageIndex);
    setState(() {
      if (selected.viewMode != null) {
        viewMode = selected.viewMode!;
      }
    });
  }

  Future<void> _openGlobalSearch(
    BuildContext context,
    ShellSnapshot snapshot,
  ) async {
    final selected = await showDialog<GlobalSearchResult>(
      context: context,
      builder: (context) => _GlobalSearchDialog(snapshot: snapshot),
    );
    if (selected == null || !mounted) return;
    _selectPage(selected.pageIndex);
  }

  List<_CommandEntry> _commandEntries(
    ShellSnapshot snapshot,
    List<_ShellPageEntry> pageEntries,
  ) {
    return [
      for (final page in pageEntries)
        _CommandEntry(
          title: '${page.label}を開く',
          subtitle: '${page.label}へ移動',
          pageIndex: page.index,
          icon: page.icon,
          keywords: page.label,
        ),
      ..._featureCommandEntries(),
      for (final mode in _ShellViewMode.values)
        _CommandEntry(
          title: '表示モードを切り替え: ${mode.label}',
          subtitle: mode.description,
          pageIndex: selectedIndex,
          icon: mode.icon,
          keywords: '表示 モード mode profile ${mode.label} ${mode.description}',
          viewMode: mode,
        ),
      for (final problem in snapshot.problems)
        _CommandEntry(
          title: problem.item.isEmpty ? problem.message : problem.item,
          subtitle: '問題 → ${problem.recoveryId}',
          pageIndex: 9,
          icon: Icons.report_problem_outlined,
          keywords:
              '${problem.problemId} ${problem.category} ${problem.classification} ${problem.target}',
          copyText: problem.target.isEmpty ? null : problem.target,
        ),
      for (final recovery in snapshot.recoveryPlaybook)
        _CommandEntry(
          title:
              recovery.recoveryId.isEmpty ? recovery.item : recovery.recoveryId,
          subtitle: recovery.requiredAction,
          pageIndex: 8,
          icon: Icons.health_and_safety_outlined,
          keywords:
              '${recovery.item} ${recovery.classification} ${recovery.command} ${recovery.path}',
          copyText: recovery.command.isNotEmpty
              ? recovery.command
              : recovery.path.isNotEmpty
                  ? recovery.path
                  : null,
        ),
      for (final runtime in snapshot.runtimes)
        _CommandEntry(
          title: runtime.runtimeId,
          subtitle: '${runtime.status}（${runtime.adapterId}経由）',
          pageIndex: 3,
          icon: Icons.hub_outlined,
          keywords: '${runtime.name} ${runtime.diagnosticSummary}',
        ),
      for (final authority in snapshot.authorityMap)
        _CommandEntry(
          title: '${authority.runtimeId} -> ${authority.capabilityId}',
          subtitle:
              '${authority.permissionId} -> ${authority.approvalId} -> ${authority.auditEventId} -> ${authority.recoveryId}',
          pageIndex: 4,
          icon: Icons.account_tree_outlined,
          keywords:
              '${authority.warning} ${authority.permissionId} ${authority.recoveryId}',
        ),
      for (final setting in snapshot.settings)
        _CommandEntry(
          title: setting.key,
          subtitle: '${setting.currentValue}（出所: ${setting.source}）',
          pageIndex: 11,
          icon: Icons.settings_outlined,
          keywords:
              '${setting.group} ${setting.effectiveValue} ${setting.authorityRelated ? 'authority' : ''} ${setting.dangerous ? 'dangerous' : ''}',
        ),
    ];
  }

  List<_CommandEntry> _featureCommandEntries() {
    return const [
      _CommandEntry(
        title: 'Runtimeを開く',
        subtitle: '実行系センターへ移動',
        pageIndex: 3,
        icon: Icons.hub_outlined,
        keywords: 'Runtime 実行系',
      ),
      _CommandEntry(
        title: 'Agentを開く',
        subtitle: 'エージェントセンターへ移動',
        pageIndex: 5,
        icon: Icons.smart_toy_outlined,
        keywords: 'Agent エージェント',
      ),
      _CommandEntry(
        title: '履歴検索',
        subtitle: '実行履歴の検索画面を開く',
        pageIndex: 13,
        icon: Icons.history,
        keywords: '履歴 実行履歴 検索',
      ),
      _CommandEntry(
        title: '評価実行',
        subtitle: '評価ラボを開く。実行はBrokerとowner承認の統治経路を使う',
        pageIndex: 14,
        icon: Icons.science_outlined,
        keywords: '評価 評価ラボ Experiment Dataset',
      ),
      _CommandEntry(
        title: 'MCP接続',
        subtitle: 'MCP接続の設定面を開く',
        pageIndex: 11,
        icon: Icons.extension_outlined,
        keywords: 'MCP 接続 道具 資源 提示',
      ),
      _CommandEntry(
        title: '通知表示',
        subtitle: '通知センターを開く',
        pageIndex: 16,
        icon: Icons.notifications_none,
        keywords: '通知 通知センター',
      ),
      _CommandEntry(
        title: '資源監視',
        subtitle: '実行系センターの資源観測面を開く',
        pageIndex: 3,
        icon: Icons.memory_outlined,
        keywords: '資源 CPU RAM 観測 実行系',
      ),
      _CommandEntry(
        title: '資格情報',
        subtitle: '資格情報の設定面を開く',
        pageIndex: 11,
        icon: Icons.key_outlined,
        keywords: '資格情報 Credential 秘密',
      ),
      _CommandEntry(
        title: '更新確認',
        subtitle: '更新センターを開く',
        pageIndex: 11,
        icon: Icons.system_update_outlined,
        keywords: '更新 Update 署名',
      ),
      _CommandEntry(
        title: 'Host切替',
        subtitle: 'Host操作面を開く。切替はBrokerの表示コンテキスト操作で行う',
        pageIndex: 19,
        icon: Icons.swap_horiz_outlined,
        keywords: 'Host 切替 接続状態 Trust',
      ),
      _CommandEntry(
        title: '全Runtime停止要求を確認',
        subtitle: '実行系センターを開き、Brokerの承認境界を確認する',
        pageIndex: 3,
        icon: Icons.stop_circle_outlined,
        keywords: '全Runtime停止要求 停止 承認 Broker',
      ),
    ];
  }
}

class _OpenCommandPaletteIntent extends Intent {
  const _OpenCommandPaletteIntent();
}

class _OpenGlobalSearchIntent extends Intent {
  const _OpenGlobalSearchIntent();
}

enum _ShellNavigationGroup {
  all,
  operation,
  safety,
  development,
  settings;

  String get label {
    return switch (this) {
      _ShellNavigationGroup.all => 'すべて',
      _ShellNavigationGroup.operation => '運用',
      _ShellNavigationGroup.safety => '安全',
      _ShellNavigationGroup.development => '開発',
      _ShellNavigationGroup.settings => '設定',
    };
  }

  bool includes(_ShellNavigationGroup pageGroup) {
    return this == _ShellNavigationGroup.all || this == pageGroup;
  }
}

class _ShellPageEntry {
  const _ShellPageEntry(this.index, this.label, this.icon, this.group);

  final int index;
  final String label;
  final IconData icon;
  final _ShellNavigationGroup group;
}

class _CommandEntry {
  const _CommandEntry({
    required this.title,
    required this.subtitle,
    required this.pageIndex,
    required this.icon,
    required this.keywords,
    this.copyText,
    this.viewMode,
  });

  final String title;
  final String subtitle;
  final int pageIndex;
  final IconData icon;
  final String keywords;
  final String? copyText;
  final _ShellViewMode? viewMode;

  bool matches(String query) {
    final normalized = query.trim().toLowerCase();
    if (normalized.isEmpty) {
      return true;
    }
    return '$title $subtitle $keywords'.toLowerCase().contains(normalized);
  }
}

class _TopCommandBar extends StatelessWidget {
  const _TopCommandBar({
    required this.selectedLabel,
    required this.viewMode,
    required this.navigationGroup,
    required this.onViewModeChanged,
    required this.onNavigationGroupChanged,
    required this.onOpenCommandPalette,
    required this.onOpenGlobalSearch,
  });

  final String selectedLabel;
  final _ShellViewMode viewMode;
  final _ShellNavigationGroup navigationGroup;
  final ValueChanged<_ShellViewMode> onViewModeChanged;
  final ValueChanged<_ShellNavigationGroup> onNavigationGroupChanged;
  final VoidCallback onOpenCommandPalette;
  final VoidCallback onOpenGlobalSearch;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: Theme.of(context).colorScheme.surface,
      child: SafeArea(
        bottom: false,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(12, 8, 12, 8),
          child: LayoutBuilder(
            builder: (context, constraints) {
              final compact = constraints.maxWidth < 920;
              return Row(
                children: [
                  Expanded(
                    child: Text(
                      selectedLabel,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Tooltip(
                    message: 'コマンドパレットを開く（Ctrl+KまたはCtrl+P）',
                    child: compact
                        ? IconButton.filled(
                            onPressed: onOpenCommandPalette,
                            icon: const Icon(Icons.search),
                          )
                        : FilledButton.icon(
                            onPressed: onOpenCommandPalette,
                            icon: const Icon(Icons.search),
                            label: const Text('コマンドパレット'),
                          ),
                  ),
                  const SizedBox(width: 8),
                  Tooltip(
                    message: '全体検索を開く（Ctrl+Shift+F）',
                    child: compact
                        ? IconButton(
                            onPressed: onOpenGlobalSearch,
                            icon: const Icon(Icons.manage_search),
                          )
                        : OutlinedButton.icon(
                            onPressed: onOpenGlobalSearch,
                            icon: const Icon(Icons.manage_search),
                            label: const Text('全体検索'),
                          ),
                  ),
                  const SizedBox(width: 8),
                  Semantics(
                    label: '操作グループ選択',
                    child: DropdownButton<_ShellNavigationGroup>(
                      value: navigationGroup,
                      underline: const SizedBox.shrink(),
                      items: [
                        for (final group in _ShellNavigationGroup.values)
                          DropdownMenuItem<_ShellNavigationGroup>(
                            value: group,
                            child: Text(group.label),
                          ),
                      ],
                      onChanged: (group) {
                        if (group != null) onNavigationGroupChanged(group);
                      },
                    ),
                  ),
                  const SizedBox(width: 8),
                  if (compact)
                    PopupMenuButton<_ShellViewMode>(
                      tooltip: '表示モード',
                      icon: Icon(viewMode.icon),
                      onSelected: onViewModeChanged,
                      itemBuilder: (context) => [
                        for (final mode in _ShellViewMode.values)
                          PopupMenuItem<_ShellViewMode>(
                            value: mode,
                            child: Text(mode.label),
                          ),
                      ],
                    )
                  else
                    SegmentedButton<_ShellViewMode>(
                      segments: [
                        for (final mode in _ShellViewMode.values)
                          ButtonSegment<_ShellViewMode>(
                            value: mode,
                            icon: Icon(mode.icon, size: 18),
                            tooltip: mode.description,
                            label: Text(mode.shortLabel),
                          ),
                      ],
                      selected: {viewMode},
                      onSelectionChanged: (selection) =>
                          onViewModeChanged(selection.single),
                    ),
                ],
              );
            },
          ),
        ),
      ),
    );
  }
}

enum _ShellViewMode {
  ownerUse,
  audit,
  releaseCandidate,
  demo;

  String get label {
    return switch (this) {
      _ShellViewMode.ownerUse => '所有者利用モード',
      _ShellViewMode.audit => '監査モード',
      _ShellViewMode.releaseCandidate => 'リリース候補モード',
      _ShellViewMode.demo => 'デモモード',
    };
  }

  String get shortLabel {
    return switch (this) {
      _ShellViewMode.ownerUse => '所有者',
      _ShellViewMode.audit => '監査',
      _ShellViewMode.releaseCandidate => 'RC',
      _ShellViewMode.demo => 'デモ',
    };
  }

  String get description {
    return switch (this) {
      _ShellViewMode.ownerUse => '日常のローカル所有者操作用表示です。',
      _ShellViewMode.audit => '証拠、監査、遮断要因を確認する表示です。',
      _ShellViewMode.releaseCandidate =>
        'リリース候補の確認用表示です。完成製品としてのリリースはまだ主張しません。',
      _ShellViewMode.demo => '読み取り専用の実演表示です。',
    };
  }

  IconData get icon {
    return switch (this) {
      _ShellViewMode.ownerUse => Icons.person_outline,
      _ShellViewMode.audit => Icons.fact_check_outlined,
      _ShellViewMode.releaseCandidate => Icons.flag_outlined,
      _ShellViewMode.demo => Icons.visibility_outlined,
    };
  }
}

class _CommandPaletteDialog extends StatefulWidget {
  const _CommandPaletteDialog({required this.entries});

  final List<_CommandEntry> entries;

  @override
  State<_CommandPaletteDialog> createState() => _CommandPaletteDialogState();
}

class _CommandPaletteDialogState extends State<_CommandPaletteDialog> {
  final TextEditingController _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final entries = widget.entries
        .where((entry) => entry.matches(_controller.text))
        .take(30)
        .toList();
    return Dialog(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 720, maxHeight: 640),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                controller: _controller,
                autofocus: true,
                decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  prefixIcon: Icon(Icons.search),
                  labelText: 'コマンドパレット',
                  helperText: '画面、問題、復旧、実行系、権限、設定を検索します',
                ),
                onChanged: (_) => setState(() {}),
                onSubmitted: (_) {
                  if (entries.isNotEmpty) {
                    Navigator.of(context).pop(entries.first);
                  }
                },
              ),
              const SizedBox(height: 12),
              Expanded(
                child: entries.isEmpty
                    ? const Center(child: Text('一致するコマンドはありません'))
                    : ListView.builder(
                        itemCount: entries.length,
                        itemBuilder: (context, index) {
                          final entry = entries[index];
                          return ListTile(
                            leading: Icon(entry.icon),
                            title: Text(entry.title),
                            subtitle: Text(entry.subtitle),
                            trailing: entry.copyText == null
                                ? null
                                : const Icon(Icons.copy, size: 18),
                            onTap: () => Navigator.of(context).pop(entry),
                          );
                        },
                      ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _GlobalSearchDialog extends StatefulWidget {
  const _GlobalSearchDialog({required this.snapshot});

  final ShellSnapshot snapshot;

  @override
  State<_GlobalSearchDialog> createState() => _GlobalSearchDialogState();
}

class _GlobalSearchDialogState extends State<_GlobalSearchDialog> {
  final TextEditingController _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final results = GlobalSearchIndex.search(
      widget.snapshot,
      _controller.text,
    );
    return Dialog(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 820, maxHeight: 680),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                controller: _controller,
                autofocus: true,
                maxLength: GlobalSearchIndex.maxQueryLength,
                decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  prefixIcon: Icon(Icons.manage_search),
                  labelText: '全体検索',
                  helperText:
                      'Runtime、Agent、Session、Auditなどを横断検索します。検索結果は表示専用です。',
                ),
                onChanged: (_) => setState(() {}),
                onSubmitted: (_) {
                  if (results.isNotEmpty) {
                    Navigator.of(context).pop(results.first);
                  }
                },
              ),
              const SizedBox(height: 12),
              Expanded(
                child: results.isEmpty
                    ? const Center(child: Text('一致する検索結果はありません'))
                    : ListView.builder(
                        itemCount: results.length,
                        itemBuilder: (context, index) {
                          final result = results[index];
                          return ListTile(
                            leading: const Icon(Icons.label_outline),
                            title: Text(result.title),
                            subtitle: Text(
                              '${result.category} / ${result.detail} / 証拠: ${result.evidenceSource}',
                            ),
                            onTap: () => Navigator.of(context).pop(result),
                          );
                        },
                      ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

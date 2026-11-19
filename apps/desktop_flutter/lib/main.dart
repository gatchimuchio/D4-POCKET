import 'dart:async';
import 'dart:ui' show PlatformDispatcher;

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter/services.dart';
import 'package:gui_shell_ui/gui_shell_ui.dart' show HistoryClient;

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
import 'screens/a2a_connection_center.dart';
import 'screens/notifications.dart';
import 'screens/observability_center.dart';
import 'screens/trace_inspector.dart';
import 'screens/settings.dart';
import 'screens/shared.dart';
import 'screens/setup_doctor.dart';
import 'screens/trust_center.dart';
import 'screens/workspace_inspector.dart' show WorkspaceInspectorHostScope;
import 'services/runtime_resource_client.dart';
import 'services/runtime_lifecycle_client.dart';
import 'services/a2a_connection_client.dart' show A2aTaskQueryResult;
import 'services/ai_edit_client.dart' show AiEditProposalReview;
import 'services/agent_task_execution_coordinator.dart';
import 'services/evaluation_client.dart';
import 'services/global_search_index.dart';
import 'services/shell_core_client.dart';
import 'services/windows_tray_client.dart';
import 'module_features.dart';

export 'module_features.dart';

const String kD4PocketProductTitle = 'D4 Pocket';
const String kGuiShellProductTitle = 'D4 Pocket powered by GUI Shell';
const double _navigationRailMinScrollableExtent = 960;

Future<void> main() async {
  await runZonedGuarded<Future<void>>(
    () async {
      WidgetsFlutterBinding.ensureInitialized();
      _installFatalErrorHandlers();
      final client = await ShellCoreClient.product();
      runApp(GuiShellDesktopApp(client: client));
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
    final initialConfiguration = client.initialUiConfiguration;
    final localeParts = initialConfiguration.locale.split('-');
    return MaterialApp(
      title: kGuiShellProductTitle,
      debugShowCheckedModeBanner: false,
      themeMode: switch (initialConfiguration.theme) {
        'system' => ThemeMode.system,
        'light' => ThemeMode.light,
        'dark' => ThemeMode.dark,
        _ => ThemeMode.system,
      },
      locale: Locale(localeParts[0], localeParts[1]),
      supportedLocales: [Locale(localeParts[0], localeParts[1])],
      localizationsDelegates: GlobalMaterialLocalizations.delegates,
      theme: _buildShellTheme(
        Brightness.light,
        density: initialConfiguration.density,
      ),
      darkTheme: _buildShellTheme(
        Brightness.dark,
        density: initialConfiguration.density,
      ),
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

ThemeData _buildShellTheme(Brightness brightness, {required String density}) {
  final scheme = ColorScheme.fromSeed(
    seedColor: const Color(0xff2f6f5e),
    brightness: brightness,
  );
  return ThemeData(
    colorScheme: scheme,
    useMaterial3: true,
    visualDensity:
        density == 'compact' ? VisualDensity.compact : VisualDensity.standard,
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
  final Set<int> _visitedPageIndexes = {0};
  bool _dialogueVisited = false;
  int _updateCenterFocusRequest = 0;
  AiEditProposalReview? _pendingAiEditProposal;
  A2aTaskQueryResult? _pendingA2aArtifactHandoff;
  _ShellViewMode viewMode = _ShellViewMode.ownerUse;
  _ShellNavigationGroup navigationGroup = _ShellNavigationGroup.all;
  late final WindowsTrayClient _trayClient;
  Timer? _trayRefreshTimer;
  bool _trayWindowVisible = false;
  bool _trayRefreshInFlight = false;
  bool _trayRefreshRequested = false;
  int _trayProjectionGeneration = 0;
  int _brokerTransportGeneration = 0;
  final ValueNotifier<int> _hostContextGeneration = ValueNotifier<int>(0);
  AgentTaskExecutionCoordinator? _agentTaskExecutionCoordinator;

  @override
  void initState() {
    super.initState();
    final transport = widget.client.brokerTransport;
    if (transport != null) {
      _agentTaskExecutionCoordinator = AgentTaskExecutionCoordinator(transport);
    }
    _trayClient = WindowsTrayClient();
    unawaited(_initializeWindowsTray());
  }

  @override
  void didUpdateWidget(covariant ShellHomePage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(
      oldWidget.client.brokerTransport,
      widget.client.brokerTransport,
    )) {
      _brokerTransportGeneration++;
      _trayProjectionGeneration++;
      _pendingAiEditProposal = null;
      _pendingA2aArtifactHandoff = null;
      _agentTaskExecutionCoordinator?.dispose();
      final transport = widget.client.brokerTransport;
      _agentTaskExecutionCoordinator =
          transport == null ? null : AgentTaskExecutionCoordinator(transport);
      if (_trayClient.available) {
        if (_trayWindowVisible) {
          unawaited(_refreshWindowsTrayProjection());
        } else {
          unawaited(_publishUnavailableWindowsTrayProjection());
        }
      }
    }
  }

  @override
  void dispose() {
    _trayRefreshTimer?.cancel();
    _agentTaskExecutionCoordinator?.dispose();
    _hostContextGeneration.dispose();
    unawaited(_trayClient.dispose());
    super.dispose();
  }

  Future<void> _initializeWindowsTray() async {
    _trayWindowVisible = await _trayClient.start(
      onAction: _handleWindowsTrayAction,
      onVisibilityChanged: (visible) {
        _setWindowsTrayVisibility(visible);
        if (visible) unawaited(_refreshWindowsTrayProjection());
      },
    );
    if (!mounted) return;
    if (!_trayClient.available) return;
    if (!_trayWindowVisible) {
      await _publishUnavailableWindowsTrayProjection();
      return;
    }
    await _refreshWindowsTrayProjection();
    _startWindowsTrayRefreshTimer();
  }

  void _startWindowsTrayRefreshTimer() {
    if (!mounted ||
        !_trayClient.available ||
        !_trayWindowVisible ||
        _trayRefreshTimer != null) {
      return;
    }
    _trayRefreshTimer = Timer.periodic(const Duration(seconds: 30), (_) {
      unawaited(_refreshWindowsTrayProjection());
    });
  }

  Future<void> _refreshWindowsTrayProjection() async {
    if (!mounted || !_trayClient.available) return;
    if (_trayRefreshInFlight) {
      _trayRefreshRequested = true;
      return;
    }
    _trayRefreshInFlight = true;
    try {
      final visibilityGeneration = _trayProjectionGeneration;
      final visibleBefore = await _trayClient.isWindowVisible();
      if (!mounted) return;
      if (visibilityGeneration != _trayProjectionGeneration) return;
      if (visibleBefore != true) {
        _setWindowsTrayVisibility(false);
        return;
      }
      if (!_trayWindowVisible) _setWindowsTrayVisibility(true);
      final generation = _trayProjectionGeneration;
      final brokerTransport = widget.client.brokerTransport;
      final projection = await WindowsTrayProjection.fromBroker(
        brokerTransport,
        windowVisible: true,
      );
      if (!mounted ||
          !_trayWindowVisible ||
          generation != _trayProjectionGeneration ||
          !identical(widget.client.brokerTransport, brokerTransport)) {
        return;
      }
      final visibleAfterGeneration = _trayProjectionGeneration;
      final visibleAfter = await _trayClient.isWindowVisible();
      if (!mounted) return;
      if (visibleAfterGeneration != _trayProjectionGeneration) return;
      if (visibleAfter != true) {
        _setWindowsTrayVisibility(false);
        return;
      }
      if (!_trayWindowVisible ||
          generation != _trayProjectionGeneration ||
          !identical(widget.client.brokerTransport, brokerTransport)) {
        return;
      }
      await _trayClient.publish(projection);
    } finally {
      _trayRefreshInFlight = false;
      final refreshRequested = _trayRefreshRequested;
      _trayRefreshRequested = false;
      if (refreshRequested && mounted && _trayClient.available) {
        if (_trayWindowVisible) {
          unawaited(_refreshWindowsTrayProjection());
        } else {
          unawaited(_publishUnavailableWindowsTrayProjection());
        }
      }
    }
  }

  void _setWindowsTrayVisibility(bool visible) {
    if (_trayWindowVisible == visible) return;
    _trayWindowVisible = visible;
    _trayProjectionGeneration += 1;
    if (visible) {
      _startWindowsTrayRefreshTimer();
      return;
    }
    _trayRefreshTimer?.cancel();
    _trayRefreshTimer = null;
    unawaited(_publishUnavailableWindowsTrayProjection());
  }

  Future<void> _publishUnavailableWindowsTrayProjection() async {
    await _trayClient.publish(
      WindowsTrayProjection.unavailable(
        stopRequestSupported: widget.client.brokerTransport != null,
      ),
    );
  }

  Future<void> _handleWindowsTrayAction(String action) async {
    if (action == 'open') {
      if (mounted) _selectPage(0);
      await _refreshWindowsTrayProjection();
      _startWindowsTrayRefreshTimer();
      return;
    }
    if (action != 'stop_request') return;
    final brokerTransport = widget.client.brokerTransport;
    final brokerGeneration = _brokerTransportGeneration;
    try {
      final result = await widget.client.requestAllRuntimeStop();
      if (!mounted) return;
      if (brokerGeneration != _brokerTransportGeneration ||
          !identical(widget.client.brokerTransport, brokerTransport)) {
        _showStaleTrayStopResult();
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(result.message)),
      );
    } on Object {
      if (!mounted) return;
      if (brokerGeneration != _brokerTransportGeneration ||
          !identical(widget.client.brokerTransport, brokerTransport)) {
        _showStaleTrayStopResult();
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('全Runtime停止要求の応答を確認できません。実行状態を再確認してください。'),
        ),
      );
    }
  }

  void _showStaleTrayStopResult() {
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(
        content: Text(
          'Broker接続先が切り替わったため、旧Brokerの停止要求結果を破棄しました。旧Brokerの状態を確認してください。',
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final snapshot = widget.client.getSnapshot();
    final historyTransport = widget.client.brokerTransport;
    _dialogueVisited = _dialogueVisited || selectedIndex == 12;
    final pages = [
      Dashboard(
        client: widget.client,
        active: selectedIndex == 0,
        onOpenProductSetup: _openProductSetup,
        onOpenAgentSetup: _openAgentSetup,
      ),
      if (kGuiShellModuleSetupDoctor)
        SetupDoctor(client: widget.client)
      else
        const SizedBox.shrink(),
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
      WorkspaceInspectorHostScope(
        generation: _hostContextGeneration,
        child: AgentCenter(
          client: widget.client,
          active: selectedIndex == 5,
          taskExecutionCoordinator: _agentTaskExecutionCoordinator,
          developerEditProposal: _pendingAiEditProposal,
          onDeveloperEditProposalPrepared: _consumeAiEditProposal,
          onDiscardDeveloperEditProposal: _discardAiEditProposal,
          a2aArtifactHandoffSource: _pendingA2aArtifactHandoff,
          onA2aArtifactHandoffConsumed: _consumeA2aArtifactHandoff,
          onDiscardA2aArtifactHandoff: _discardA2aArtifactHandoff,
        ),
      ),
      ApprovalCenter(client: widget.client),
      AuditViewer(client: widget.client),
      RecoveryCenter(client: widget.client, active: selectedIndex == 8),
      ProblemsPanel(client: widget.client, active: selectedIndex == 9),
      EvidenceCenter(client: widget.client),
      WorkspaceInspectorHostScope(
        generation: _hostContextGeneration,
        child: SettingsScreen(
          client: widget.client,
          active: selectedIndex == 11,
          focusUpdateCenterRequest: _updateCenterFocusRequest,
          onPrepareAiEditTask: _openAiEditProposal,
          onBrokerMutationOutcomeUnknown: _reportBrokerMutationOutcomeUnknown,
        ),
      ),
      if (_dialogueVisited)
        RuntimeDialogueScreen(
          active: selectedIndex == 12,
          readOnly: viewMode == _ShellViewMode.demo,
        )
      else
        const SizedBox.shrink(),
      if (kGuiShellModuleHistory &&
          selectedIndex == 13 &&
          viewMode == _ShellViewMode.ownerUse)
        HistoryScreen(
          client:
              historyTransport == null ? null : HistoryClient(historyTransport),
          hostContextGeneration: _hostContextGeneration,
        )
      else
        const SizedBox.shrink(),
      if (kGuiShellModuleEvaluationLab)
        EvaluationLab(
          client: widget.evaluationClient,
          connect: widget.evaluationConnect,
          shellCoreClient: widget.client,
          hostContextGeneration: _hostContextGeneration,
        )
      else
        const SizedBox.shrink(),
      if (kGuiShellModuleHostCapabilities)
        HostCapabilityCenter(client: widget.client)
      else
        const SizedBox.shrink(),
      if (kGuiShellModuleNotifications)
        NotificationsScreen(
          client: widget.client,
          onNavigate: _navigateFromNotification,
        )
      else
        const SizedBox.shrink(),
      if (kGuiShellModuleObservability)
        ObservabilityCenter(client: widget.client)
      else
        const SizedBox.shrink(),
      if (kGuiShellModuleTraceInspector)
        TraceInspector(client: widget.client)
      else
        const SizedBox.shrink(),
      if (kGuiShellModuleHostOperations)
        HostOperationCenter(
          client: widget.client,
          onHostContextChanged: () => _hostContextGeneration.value++,
        )
      else
        const SizedBox.shrink(),
      A2aConnectionCenter(
        transport: widget.client.brokerTransport,
        onPrepareAgentTask: _openA2aArtifactHandoff,
      ),
    ];
    final retainedPages = List<Widget>.generate(
      pages.length,
      (index) => _visitedPageIndexes.contains(index)
          ? pages[index]
          : const SizedBox.shrink(),
      growable: false,
    );
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
              _openCommandPalette(context);
              return null;
            },
          ),
          _OpenGlobalSearchIntent: CallbackAction<_OpenGlobalSearchIntent>(
            onInvoke: (_) {
              _openGlobalSearch(context);
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
                  selectedLabel: pageEntries
                      .firstWhere((page) => page.index == selectedIndex)
                      .label,
                  viewMode: viewMode,
                  navigationGroup: navigationGroup,
                  onViewModeChanged: (mode) => setState(() => viewMode = mode),
                  onNavigationGroupChanged: _setNavigationGroup,
                  onOpenCommandPalette: () => _openCommandPalette(context),
                  onOpenGlobalSearch: () => _openGlobalSearch(context),
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
                                          if (kGuiShellModuleSetupDoctor)
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
                                          if (kGuiShellModuleHistory)
                                            NavigationRailDestination(
                                              icon:
                                                  Icon(Icons.history_outlined),
                                              selectedIcon: Icon(Icons.history),
                                              label: Text('履歴'),
                                            ),
                                          if (kGuiShellModuleEvaluationLab)
                                            NavigationRailDestination(
                                              icon:
                                                  Icon(Icons.science_outlined),
                                              selectedIcon: Icon(Icons.science),
                                              label: Text('評価ラボ'),
                                            ),
                                          if (kGuiShellModuleHostCapabilities)
                                            NavigationRailDestination(
                                              icon: Icon(Icons.public_outlined),
                                              selectedIcon: Icon(Icons.public),
                                              label: Text('ホスト能力'),
                                            ),
                                          if (kGuiShellModuleNotifications)
                                            NavigationRailDestination(
                                              icon: Icon(
                                                  Icons.notifications_none),
                                              selectedIcon:
                                                  Icon(Icons.notifications),
                                              label: Text('通知'),
                                            ),
                                          if (kGuiShellModuleObservability)
                                            NavigationRailDestination(
                                              icon:
                                                  Icon(Icons.insights_outlined),
                                              selectedIcon:
                                                  Icon(Icons.insights),
                                              label: Text('観測'),
                                            ),
                                          if (kGuiShellModuleTraceInspector)
                                            NavigationRailDestination(
                                              icon:
                                                  Icon(Icons.timeline_outlined),
                                              selectedIcon:
                                                  Icon(Icons.timeline),
                                              label: Text('追跡'),
                                            ),
                                          if (kGuiShellModuleHostOperations)
                                            NavigationRailDestination(
                                              icon: Icon(
                                                  Icons.swap_horiz_outlined),
                                              selectedIcon:
                                                  Icon(Icons.swap_horiz),
                                              label: Text('Host操作'),
                                            ),
                                          NavigationRailDestination(
                                            icon: Icon(Icons.hub_outlined),
                                            selectedIcon: Icon(Icons.hub),
                                            label: Text('A2A接続'),
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
                              index: selectedIndex, children: retainedPages)),
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
      if (kGuiShellModuleSetupDoctor)
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
      if (kGuiShellModuleHistory)
        _ShellPageEntry(
            13, '実行履歴', Icons.history, _ShellNavigationGroup.operation),
      if (kGuiShellModuleEvaluationLab)
        _ShellPageEntry(14, '評価ラボ', Icons.science_outlined,
            _ShellNavigationGroup.operation),
      if (kGuiShellModuleHostCapabilities)
        _ShellPageEntry(15, 'ホスト能力', Icons.public_outlined,
            _ShellNavigationGroup.operation),
      if (kGuiShellModuleNotifications)
        _ShellPageEntry(16, '通知センター', Icons.notifications_none,
            _ShellNavigationGroup.operation),
      if (kGuiShellModuleObservability)
        _ShellPageEntry(17, '観測センター', Icons.insights_outlined,
            _ShellNavigationGroup.operation),
      if (kGuiShellModuleTraceInspector)
        _ShellPageEntry(18, '追跡情報', Icons.timeline_outlined,
            _ShellNavigationGroup.development),
      if (kGuiShellModuleHostOperations)
        _ShellPageEntry(19, 'Host操作面', Icons.swap_horiz_outlined,
            _ShellNavigationGroup.operation),
      _ShellPageEntry(
          20, 'A2A接続', Icons.hub_outlined, _ShellNavigationGroup.operation),
    ];
  }

  void _navigateFromNotification(String target) {
    final index = switch (target) {
      'approval' => 6,
      'permission' => 4,
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

  void _openProductSetup() {
    setState(() {
      selectedIndex = 11;
      _visitedPageIndexes.add(selectedIndex);
      navigationGroup = _ShellNavigationGroup.settings;
      _updateCenterFocusRequest++;
    });
  }

  void _openAgentSetup() => _selectPage(5);

  void _openAiEditProposal(AiEditProposalReview proposal) {
    final matches = _pageEntries().where((entry) => entry.index == 5);
    if (matches.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('この書出し構成にはAgent Centerがありません。')),
      );
      return;
    }
    final page = matches.first;
    setState(() {
      _pendingAiEditProposal = proposal;
      selectedIndex = 5;
      _visitedPageIndexes.add(selectedIndex);
      if (!navigationGroup.includes(page.group)) {
        navigationGroup = page.group;
      }
    });
  }

  void _consumeAiEditProposal(AiEditProposalReview proposal) {
    if (!identical(_pendingAiEditProposal, proposal)) return;
    setState(() => _pendingAiEditProposal = null);
  }

  void _discardAiEditProposal() {
    if (_pendingAiEditProposal == null) return;
    setState(() => _pendingAiEditProposal = null);
  }

  void _openA2aArtifactHandoff(A2aTaskQueryResult source) {
    final matches = _pageEntries().where((entry) => entry.index == 5);
    if (matches.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Agent CenterがないためArtifactを渡せません。')),
      );
      return;
    }
    final page = matches.first;
    setState(() {
      _pendingA2aArtifactHandoff = source;
      selectedIndex = page.index;
      _visitedPageIndexes.add(selectedIndex);
      if (!navigationGroup.includes(page.group)) {
        navigationGroup = page.group;
      }
    });
  }

  void _consumeA2aArtifactHandoff(A2aTaskQueryResult source) {
    if (!identical(_pendingA2aArtifactHandoff, source)) return;
    setState(() => _pendingA2aArtifactHandoff = null);
  }

  void _discardA2aArtifactHandoff() {
    if (_pendingA2aArtifactHandoff == null) return;
    setState(() => _pendingA2aArtifactHandoff = null);
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
        _visitedPageIndexes.add(selectedIndex);
      }
    });
  }

  void _selectPage(int index) {
    final matches = _pageEntries().where((entry) => entry.index == index);
    if (matches.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('この書出し構成には対象Moduleが含まれていません。')),
      );
      return;
    }
    final page = matches.first;
    setState(() {
      selectedIndex = index;
      _visitedPageIndexes.add(index);
      if (!navigationGroup.includes(page.group)) {
        navigationGroup = page.group;
      }
    });
  }

  Future<void> _openCommandPalette(BuildContext context) =>
      _openCurrentBrokerSearch(context, commandPalette: true);

  Future<void> _openGlobalSearch(BuildContext context) =>
      _openCurrentBrokerSearch(context, commandPalette: false);

  Future<void> _openCurrentBrokerSearch(
    BuildContext context, {
    required bool commandPalette,
  }) async {
    final broker = widget.client.brokerTransport;
    if (broker == null) {
      _showCurrentSearchUnavailable(context);
      return;
    }
    final brokerGeneration = _brokerTransportGeneration;

    late final List<GlobalSearchResult> entries;
    try {
      entries = await GlobalSearchIndex.loadCurrent(broker);
    } on Object {
      if (context.mounted) _showCurrentSearchUnavailable(context);
      return;
    }
    if (!mounted || !context.mounted) return;
    if (brokerGeneration != _brokerTransportGeneration ||
        !identical(widget.client.brokerTransport, broker)) {
      _showSearchInvalidated(context);
      return;
    }

    final availablePageIndices =
        _pageEntries().map((page) => page.index).toSet();
    final visibleEntries = entries
        .where((entry) => availablePageIndices.contains(entry.pageIndex))
        .toList(growable: false);
    final selected = await showDialog<GlobalSearchResult>(
      context: context,
      builder: (context) => _CurrentBrokerSearchDialog(
        entries: visibleEntries,
        label: commandPalette ? 'コマンドパレット' : '全体検索',
      ),
    );
    if (selected == null || !mounted || !context.mounted) return;
    if (brokerGeneration != _brokerTransportGeneration ||
        !identical(widget.client.brokerTransport, broker)) {
      _showSearchInvalidated(context);
      return;
    }

    // Search and palette selections navigate only. They do not pass the
    // indexed record or any authority to the destination.
    _selectPage(selected.pageIndex);
  }

  void _showCurrentSearchUnavailable(BuildContext context) {
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(
        content: Text('Brokerの現在Host／Workspace状態を取得できません。検索結果を表示していません。'),
      ),
    );
  }

  void _showSearchInvalidated(BuildContext context) {
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(
        content: Text('Broker接続先が切り替わったため、古い検索結果を破棄しました。再度検索してください。'),
      ),
    );
  }

  void _reportBrokerMutationOutcomeUnknown() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text(
            'Broker接続先が切り替わりました。切替前のBroker作用の結果は不明です。自動再送していません。現在の画面から状態を再取得し、必要なら旧Brokerの監査と対象状態を確認してください。',
          ),
        ),
      );
    });
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

class _CurrentBrokerSearchDialog extends StatefulWidget {
  const _CurrentBrokerSearchDialog({
    required this.entries,
    required this.label,
  });

  final List<GlobalSearchResult> entries;
  final String label;

  @override
  State<_CurrentBrokerSearchDialog> createState() =>
      _CurrentBrokerSearchDialogState();
}

class _CurrentBrokerSearchDialogState
    extends State<_CurrentBrokerSearchDialog> {
  final TextEditingController _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final results = GlobalSearchIndex.search(widget.entries, _controller.text);
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
                decoration: InputDecoration(
                  border: const OutlineInputBorder(),
                  prefixIcon: const Icon(Icons.manage_search),
                  labelText: widget.label,
                  helperText:
                      'Brokerの現在Host・Workspaceと有効なWorkspace読取だけを検索します。選択時は画面移動のみです。',
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
                    ? const Center(child: Text('一致する現在の項目はありません'))
                    : ListView.builder(
                        itemCount: results.length,
                        itemBuilder: (context, index) {
                          final result = results[index];
                          return ListTile(
                            leading: const Icon(Icons.label_outline),
                            title: Text(result.title),
                            subtitle: Text(
                              '${result.category} / ${result.detail} / '
                              '証拠: ${result.evidenceSource}',
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

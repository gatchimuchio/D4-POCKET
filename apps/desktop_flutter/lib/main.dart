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
import 'screens/agent_center.dart';
import 'screens/authority_map.dart';
import 'screens/problems_panel.dart';
import 'screens/recovery_center.dart';
import 'screens/runtime_center.dart';
import 'screens/settings.dart';
import 'screens/shared.dart';
import 'screens/setup_doctor.dart';
import 'screens/trust_center.dart';
import 'services/setup_doctor_export.dart';
import 'services/shell_core_client.dart';
import 'services/surface_semantics_export.dart';

const String kGuiShellProductTitle = 'GUI Shell';
const double _navigationRailMinScrollableExtent = 760;

SemanticsHandle? _appSemanticsHandle;

Future<void> main() async {
  await runZonedGuarded<Future<void>>(
    () async {
      WidgetsFlutterBinding.ensureInitialized();
      _installFatalErrorHandlers();
      _ensureAccessibilitySemantics();
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
          context: ErrorDescription('uncaught app zone error'),
        ),
      );
    },
  );
}

void _ensureAccessibilitySemantics() {
  _appSemanticsHandle ??= SemanticsBinding.instance.ensureSemantics();
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
        context: ErrorDescription('uncaught platform dispatcher error'),
      ),
    );
    return true;
  };
  ErrorWidget.builder = (details) =>
      GuiShellFatalErrorScreen(message: details.exceptionAsString());
}

class GuiShellDesktopApp extends StatelessWidget {
  const GuiShellDesktopApp({super.key, this.client});

  final ShellCoreClient? client;

  @override
  Widget build(BuildContext context) {
    final client = this.client ?? ShellCoreClient.mock();
    return MaterialApp(
      title: kGuiShellProductTitle,
      debugShowCheckedModeBanner: false,
      themeMode: ThemeMode.system,
      theme: _buildShellTheme(Brightness.light),
      darkTheme: _buildShellTheme(Brightness.dark),
      home: ShellHomePage(client: client),
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
                    'GUI Shell encountered a fatal UI error.',
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
  const ShellHomePage({super.key, required this.client});

  final ShellCoreClient client;

  @override
  State<ShellHomePage> createState() => _ShellHomePageState();
}

class _ShellHomePageState extends State<ShellHomePage> {
  int selectedIndex = 0;
  _ShellViewMode viewMode = _ShellViewMode.ownerUse;

  @override
  Widget build(BuildContext context) {
    final snapshot = widget.client.getSnapshot();
    final pages = [
      Dashboard(client: widget.client),
      SetupDoctor(client: widget.client),
      TrustCenter(client: widget.client),
      RuntimeCenter(client: widget.client),
      AuthorityMap(client: widget.client),
      AgentCenter(client: widget.client),
      ApprovalCenter(client: widget.client),
      AuditViewer(client: widget.client),
      RecoveryCenter(client: widget.client),
      ProblemsPanel(client: widget.client),
      EvidenceCenter(client: widget.client),
      SettingsScreen(client: widget.client),
    ];
    final pageEntries = _pageEntries();

    return Shortcuts(
      shortcuts: const {
        SingleActivator(LogicalKeyboardKey.keyK, control: true):
            _OpenCommandPaletteIntent(),
        SingleActivator(LogicalKeyboardKey.keyP, control: true):
            _OpenCommandPaletteIntent(),
      },
      child: Actions(
        actions: {
          _OpenCommandPaletteIntent: CallbackAction<_OpenCommandPaletteIntent>(
            onInvoke: (_) {
              _openCommandPalette(context, snapshot, pageEntries);
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
                  onViewModeChanged: (mode) => setState(() => viewMode = mode),
                  onOpenCommandPalette: () =>
                      _openCommandPalette(context, snapshot, pageEntries),
                ),
                PhaseBanner(snapshot: snapshot),
                Expanded(
                  child: Row(
                    children: [
                      SurfaceSemantics(
                        label: 'NavigationRail',
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
                                  selectedIndex: selectedIndex,
                                  onDestinationSelected: (index) =>
                                      setState(() => selectedIndex = index),
                                  labelType: NavigationRailLabelType.selected,
                                  destinations: const [
                                    NavigationRailDestination(
                                        icon: Icon(Icons.dashboard_outlined),
                                        selectedIcon: Icon(Icons.dashboard),
                                        label: Text('Dashboard')),
                                    NavigationRailDestination(
                                        icon: Icon(Icons.build_circle_outlined),
                                        selectedIcon: Icon(Icons.build_circle),
                                        label: Text('Doctor')),
                                    NavigationRailDestination(
                                        icon:
                                            Icon(Icons.verified_user_outlined),
                                        selectedIcon: Icon(Icons.verified_user),
                                        label: Text('Trust')),
                                    NavigationRailDestination(
                                        icon: Icon(Icons.hub_outlined),
                                        selectedIcon: Icon(Icons.hub),
                                        label: Text('Runtime')),
                                    NavigationRailDestination(
                                        icon: Icon(Icons.account_tree_outlined),
                                        selectedIcon: Icon(Icons.account_tree),
                                        label: Text('Authority')),
                                    NavigationRailDestination(
                                        icon: Icon(Icons.smart_toy_outlined),
                                        selectedIcon: Icon(Icons.smart_toy),
                                        label: Text('Agent')),
                                    NavigationRailDestination(
                                        icon: Icon(Icons.fact_check_outlined),
                                        selectedIcon: Icon(Icons.fact_check),
                                        label: Text('Approval')),
                                    NavigationRailDestination(
                                        icon: Icon(Icons.receipt_long_outlined),
                                        selectedIcon: Icon(Icons.receipt_long),
                                        label: Text('Audit')),
                                    NavigationRailDestination(
                                        icon: Icon(
                                            Icons.health_and_safety_outlined),
                                        selectedIcon:
                                            Icon(Icons.health_and_safety),
                                        label: Text('Recovery')),
                                    NavigationRailDestination(
                                        icon:
                                            Icon(Icons.report_problem_outlined),
                                        selectedIcon:
                                            Icon(Icons.report_problem),
                                        label: Text('Problems')),
                                    NavigationRailDestination(
                                        icon: Icon(Icons.inventory_2_outlined),
                                        selectedIcon: Icon(Icons.inventory_2),
                                        label: Text('Evidence')),
                                    NavigationRailDestination(
                                        icon: Icon(Icons.settings_outlined),
                                        selectedIcon: Icon(Icons.settings),
                                        label: Text('Settings')),
                                  ],
                                ),
                              ),
                            );
                          },
                        ),
                      ),
                      const VerticalDivider(width: 1),
                      Expanded(child: pages[selectedIndex]),
                    ],
                  ),
                ),
                ShellStatusBar(snapshot: snapshot),
              ],
            ),
          ),
        ),
      ),
    );
  }

  List<_ShellPageEntry> _pageEntries() {
    return const [
      _ShellPageEntry(0, 'Dashboard', Icons.dashboard_outlined),
      _ShellPageEntry(1, 'Setup Doctor', Icons.build_circle_outlined),
      _ShellPageEntry(2, 'Trust Center', Icons.verified_user_outlined),
      _ShellPageEntry(3, 'Runtime Center', Icons.hub_outlined),
      _ShellPageEntry(4, 'Authority Map', Icons.account_tree_outlined),
      _ShellPageEntry(5, 'Agent Center', Icons.smart_toy_outlined),
      _ShellPageEntry(6, 'Approval Center', Icons.fact_check_outlined),
      _ShellPageEntry(7, 'Audit Viewer', Icons.receipt_long_outlined),
      _ShellPageEntry(8, 'Recovery Playbook', Icons.health_and_safety_outlined),
      _ShellPageEntry(9, 'Problems Panel', Icons.report_problem_outlined),
      _ShellPageEntry(10, 'Evidence Center', Icons.inventory_2_outlined),
      _ShellPageEntry(11, 'Settings', Icons.settings_outlined),
    ];
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
    setState(() {
      selectedIndex = selected.pageIndex;
      if (selected.viewMode != null) {
        viewMode = selected.viewMode!;
      }
    });
  }

  List<_CommandEntry> _commandEntries(
    ShellSnapshot snapshot,
    List<_ShellPageEntry> pageEntries,
  ) {
    return [
      for (final page in pageEntries)
        _CommandEntry(
          title: 'Open ${page.label}',
          subtitle: 'Navigate to ${page.label}',
          pageIndex: page.index,
          icon: page.icon,
          keywords: page.label,
        ),
      for (final mode in _ShellViewMode.values)
        _CommandEntry(
          title: 'Switch view mode: ${mode.label}',
          subtitle: mode.description,
          pageIndex: selectedIndex,
          icon: mode.icon,
          keywords: 'mode profile ${mode.label} ${mode.description}',
          viewMode: mode,
        ),
      for (final problem in snapshot.problems)
        _CommandEntry(
          title: problem.item.isEmpty ? problem.message : problem.item,
          subtitle: 'Problem -> ${problem.recoveryId}',
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
          subtitle: '${runtime.status} via ${runtime.adapterId}',
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
          subtitle: '${setting.currentValue} from ${setting.source}',
          pageIndex: 11,
          icon: Icons.settings_outlined,
          keywords:
              '${setting.group} ${setting.effectiveValue} ${setting.authorityRelated ? 'authority' : ''} ${setting.dangerous ? 'dangerous' : ''}',
        ),
    ];
  }
}

class _OpenCommandPaletteIntent extends Intent {
  const _OpenCommandPaletteIntent();
}

class _ShellPageEntry {
  const _ShellPageEntry(this.index, this.label, this.icon);

  final int index;
  final String label;
  final IconData icon;
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
    required this.onViewModeChanged,
    required this.onOpenCommandPalette,
  });

  final String selectedLabel;
  final _ShellViewMode viewMode;
  final ValueChanged<_ShellViewMode> onViewModeChanged;
  final VoidCallback onOpenCommandPalette;

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
                    message: 'Open command palette (Ctrl+K or Ctrl+P)',
                    child: compact
                        ? IconButton.filled(
                            onPressed: onOpenCommandPalette,
                            icon: const Icon(Icons.search),
                          )
                        : FilledButton.icon(
                            onPressed: onOpenCommandPalette,
                            icon: const Icon(Icons.search),
                            label: const Text('Command Palette'),
                          ),
                  ),
                  const SizedBox(width: 8),
                  if (compact)
                    PopupMenuButton<_ShellViewMode>(
                      tooltip: 'View mode',
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
      _ShellViewMode.ownerUse => 'Owner-use mode',
      _ShellViewMode.audit => 'Audit mode',
      _ShellViewMode.releaseCandidate => 'Release candidate mode',
      _ShellViewMode.demo => 'Demo mode',
    };
  }

  String get shortLabel {
    return switch (this) {
      _ShellViewMode.ownerUse => 'Owner',
      _ShellViewMode.audit => 'Audit',
      _ShellViewMode.releaseCandidate => 'RC',
      _ShellViewMode.demo => 'Demo',
    };
  }

  String get description {
    return switch (this) {
      _ShellViewMode.ownerUse => 'Daily local owner operation view.',
      _ShellViewMode.audit => 'Evidence, audit, and blocker review view.',
      _ShellViewMode.releaseCandidate =>
        'RC review view; completed product release remains not claimed.',
      _ShellViewMode.demo => 'Read-only demonstration view.',
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
                  labelText: 'Command Palette',
                  helperText:
                      'Search pages, problems, recovery, runtime, authority, and settings',
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
                    ? const Center(child: Text('No matching command'))
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

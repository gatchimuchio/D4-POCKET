import 'dart:async';
import 'package:flutter/material.dart';
import 'package:gui_shell_ui/gui_shell_ui.dart';
import 'screens/approval_review.dart';
import 'screens/agent_status.dart';
import 'screens/device_connection.dart';
import 'screens/emergency_stop.dart';
import 'screens/mobile_dashboard.dart';
import 'screens/notifications.dart';
import 'screens/recovery_instruction.dart';
import 'screens/resource_overview.dart';
import 'screens/runtime_status.dart';
import 'screens/history.dart';
import 'screens/mcp_status.dart';
import 'services/device_link_controller.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const GuiShellMobileApp());
}

class GuiShellMobileApp extends StatelessWidget {
  const GuiShellMobileApp({super.key, this.controller});
  final DeviceLinkController? controller;
  @override
  Widget build(BuildContext context) => MaterialApp(
    title: 'GUI Shell モバイル',
    debugShowCheckedModeBanner: false,
    theme: ThemeData(
      colorSchemeSeed: const Color(0xff2f6f5e),
      useMaterial3: true,
    ),
    home: MobileHome(controller: controller),
  );
}

class MobileHome extends StatefulWidget {
  const MobileHome({super.key, this.controller});
  final DeviceLinkController? controller;
  @override
  State<MobileHome> createState() => _MobileHomeState();
}

class _MobileHomeState extends State<MobileHome> with WidgetsBindingObserver {
  late final DeviceLinkController _controller;
  int _selected = 0;
  bool _dialogueVisited = false;
  static const _names = [
    '概要',
    'Agent',
    '確認',
    '通知',
    '実行系',
    '停止',
    '復旧',
    '対話',
    '接続先',
    '設定',
    '資源',
    '履歴',
    'MCP',
  ];
  static const _icons = [
    Icons.dashboard_outlined,
    Icons.smart_toy_outlined,
    Icons.fact_check_outlined,
    Icons.notifications_outlined,
    Icons.hub_outlined,
    Icons.stop_circle_outlined,
    Icons.health_and_safety_outlined,
    Icons.chat_outlined,
    Icons.link,
    Icons.settings_outlined,
    Icons.memory_outlined,
    Icons.history_outlined,
    Icons.extension_outlined,
  ];
  @override
  void initState() {
    super.initState();
    _controller = widget.controller ?? DeviceLinkController();
    WidgetsBinding.instance.addObserver(this);
    final state = WidgetsBinding.instance.lifecycleState;
    if (state != null && state != AppLifecycleState.resumed)
      _controller.setForeground(false);
    unawaited(_controller.initialize());
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) =>
      _controller.setForeground(state == AppLifecycleState.resumed);
  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    if (widget.controller == null) _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: _controller,
    builder: (context, _) {
      final c = _controller;
      final pages = <Widget>[
        MobileDashboard(status: c.status),
        MobileAgentStatus(
          controller: c,
          connected: c.ready && c.foreground,
          active: _selected == 1,
        ),
        const ApprovalReview(),
        MobileNotifications(
          events: c.events,
          controller: c,
          connected: c.ready && c.foreground,
        ),
        RuntimeStatus(
          controller: c,
          runtimes: c.runtimes,
          connected: c.ready && c.foreground,
        ),
        EmergencyStop(controller: c),
        const RecoveryInstruction(),
        if (_dialogueVisited)
          RuntimeDialogueScreen(
            key: ValueKey(c.connectionGeneration),
            connect: () async => c.dialogue,
            active: c.ready && c.foreground && _selected == 7,
            workspaceSelectionSupported: true,
          )
        else
          const SizedBox.shrink(),
        DeviceConnection(controller: c),
        DeviceSettings(controller: c),
        ResourceOverview(
          controller: c,
          runtimes: c.runtimes,
          connected: c.ready && c.foreground,
        ),
        MobileHistory(controller: c, connected: c.ready && c.foreground),
        const MobileMcpStatus(),
      ];
      return Scaffold(
        appBar: AppBar(
          title: Text('GUI Shell・${_names[_selected]}'),
          actions: [
            Padding(
              padding: const EdgeInsets.all(12),
              child: Icon(c.ready ? Icons.link : Icons.link_off),
            ),
          ],
        ),
        drawer: NavigationDrawer(
          selectedIndex: _selected,
          onDestinationSelected: (index) {
            setState(() {
              _selected = index;
              _dialogueVisited |= index == 7;
            });
            Navigator.pop(context);
          },
          children: [
            const Padding(
              padding: EdgeInsets.all(24),
              child: Text('GUI Shell モバイル'),
            ),
            for (var i = 0; i < _names.length; i++)
              NavigationDrawerDestination(
                icon: Icon(_icons[i]),
                label: Text(_names[i]),
              ),
          ],
        ),
        body: IndexedStack(index: _selected, children: pages),
      );
    },
  );
}

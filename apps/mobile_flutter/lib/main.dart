import 'package:flutter/material.dart';

import 'screens/approval_review.dart';
import 'screens/emergency_stop.dart';
import 'screens/mobile_dashboard.dart';
import 'screens/notifications.dart';
import 'screens/recovery_instruction.dart';
import 'screens/runtime_status.dart';

void main() {
  runApp(const GuiShellMobileApp());
}

class GuiShellMobileApp extends StatelessWidget {
  const GuiShellMobileApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'GUI Shell モバイル',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorSchemeSeed: const Color(0xff2f6f5e),
        useMaterial3: true,
      ),
      home: const MobileHome(),
    );
  }
}

class MobileHome extends StatefulWidget {
  const MobileHome({super.key});

  @override
  State<MobileHome> createState() => _MobileHomeState();
}

class _MobileHomeState extends State<MobileHome> {
  int selectedIndex = 0;

  @override
  Widget build(BuildContext context) {
    const pages = [
      MobileDashboard(),
      ApprovalReview(),
      MobileNotifications(),
      RuntimeStatus(),
      EmergencyStop(),
      RecoveryInstruction(),
    ];
    return Scaffold(
      body: pages[selectedIndex],
      bottomNavigationBar: NavigationBar(
        selectedIndex: selectedIndex,
        onDestinationSelected: (index) => setState(() => selectedIndex = index),
        destinations: const [
          NavigationDestination(
            icon: Icon(Icons.dashboard_outlined),
            selectedIcon: Icon(Icons.dashboard),
            label: '概要',
          ),
          NavigationDestination(
            icon: Icon(Icons.fact_check_outlined),
            selectedIcon: Icon(Icons.fact_check),
            label: '確認',
          ),
          NavigationDestination(
            icon: Icon(Icons.notifications_outlined),
            selectedIcon: Icon(Icons.notifications),
            label: '通知',
          ),
          NavigationDestination(
            icon: Icon(Icons.hub_outlined),
            selectedIcon: Icon(Icons.hub),
            label: '実行系',
          ),
          NavigationDestination(
            icon: Icon(Icons.stop_circle_outlined),
            selectedIcon: Icon(Icons.stop_circle),
            label: '停止',
          ),
          NavigationDestination(
            icon: Icon(Icons.health_and_safety_outlined),
            selectedIcon: Icon(Icons.health_and_safety),
            label: '復旧',
          ),
        ],
      ),
    );
  }
}

/// Export Module selection compiled by each platform's Product exporter.
///
/// These flags select product surfaces. Rust Broker independently enforces
/// the matching Cargo feature set at its authenticated IPC boundary.
const bool kGuiShellModuleSetupDoctor =
    bool.fromEnvironment('GUI_SHELL_MODULE_SETUP_DOCTOR', defaultValue: true);
const bool kGuiShellModuleHistory =
    bool.fromEnvironment('GUI_SHELL_MODULE_HISTORY', defaultValue: true);
const bool kGuiShellModuleEvaluationLab =
    bool.fromEnvironment('GUI_SHELL_MODULE_EVALUATION_LAB', defaultValue: true);
const bool kGuiShellModuleHostCapabilities = bool.fromEnvironment(
    'GUI_SHELL_MODULE_HOST_CAPABILITIES',
    defaultValue: true);
const bool kGuiShellModuleNotifications =
    bool.fromEnvironment('GUI_SHELL_MODULE_NOTIFICATIONS', defaultValue: true);
const bool kGuiShellModuleObservability =
    bool.fromEnvironment('GUI_SHELL_MODULE_OBSERVABILITY', defaultValue: true);
const bool kGuiShellModuleTraceInspector = bool.fromEnvironment(
        'GUI_SHELL_MODULE_TRACE_INSPECTOR',
        defaultValue: true) &&
    kGuiShellModuleObservability;
const bool kGuiShellModuleHostOperations = bool.fromEnvironment(
    'GUI_SHELL_MODULE_HOST_OPERATIONS',
    defaultValue: true);

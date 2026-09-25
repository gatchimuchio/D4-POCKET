import 'package:gui_shell_ui/gui_shell_ui.dart' as shared;
import '../services/regression_case_client.dart';
import '../services/runtime_dialogue_client.dart';

/// Desktop固有の通常broker接続だけを共通表示へ渡す。
class RuntimeDialogueScreen extends shared.RuntimeDialogueScreen {
  const RuntimeDialogueScreen({super.key, super.client, super.readOnly})
      : super(
          connect: connectRuntimeDialogue,
          connectOwnerRegistration: connectRegressionCaseOwnerClient,
        );
}

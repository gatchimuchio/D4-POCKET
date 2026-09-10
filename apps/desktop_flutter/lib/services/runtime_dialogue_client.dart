import 'package:gui_shell_ui/runtime_dialogue_client.dart';
import 'broker_client.dart';
export 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show RuntimeDialogueClient, DialogueProgress, DialogueResult;

Future<RuntimeDialogueClient> connectRuntimeDialogue() async =>
    RuntimeDialogueClient(await BrokerClient.connect());

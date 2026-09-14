import 'package:gui_shell_ui/evaluation_client.dart';

import 'broker_client.dart';

export 'package:gui_shell_ui/evaluation_client.dart'
    show
        EvaluationCaseSummary,
        EvaluationClient,
        EvaluationComparison,
        EvaluationComparisonExperiment,
        EvaluationDataset,
        EvaluationDatasetListing,
        EvaluationEvaluatorJudgement,
        EvaluationExperiment,
        EvaluationExperimentStatus,
        EvaluationResultSummary;

typedef EvaluationClientConnector = Future<EvaluationClient> Function();

/// Desktopの通常資格で、公開済みの評価projectionだけをBrokerへ要求する。
/// owner操作や非公開Dataset payloadはこの接続面へ渡さない。
Future<EvaluationClient> connectEvaluationClient() async =>
    EvaluationClient(await BrokerClient.connect());

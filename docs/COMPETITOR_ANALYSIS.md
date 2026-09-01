# 競合製品分析

確認した情報源:

- Docker Desktop 文書: https://docs.docker.com/desktop/
- AnythingLLM Desktop 文書: https://docs.anythingllm.com/installation-desktop/overview
- OpenHands クイックスタート文書: https://docs.openhands.dev/overview/quickstart
- LM Studio 文書: https://lmstudio.ai/docs
- Open WebUI 文書: https://docs.openwebui.com/
- Ollama 文書: https://docs.ollama.com/
- AIDev 論文: https://arxiv.org/abs/2602.09185

## 分類済み要求

~~~yaml
- requirement: デスクトップGUIは下位層のランタイム複雑性を隠しつつ、インストール、設定、ログ、トラブルシューティング、バックアップ、セキュリティ、およびAI・エージェント操作面を公開する。
  source/competitor: Docker Desktop
  implementation target: Dashboard、Setup Doctor、Runtime Center、Agent Center、Audit Viewer、Recovery Center
  classification: required_for_v1
  blocks_release: yes

- requirement: 導入時の負担が少ない、単一ユーザー向けローカル優先デスクトップ製品とする。
  source/competitor: AnythingLLM Desktop
  implementation target: デスクトップアプリとインストーラー初回実行フロー
  classification: required_for_v1
  blocks_release: yes

- requirement: 通常ユーザーにDockerやCLIのセットアップを強制せず、ローカルGUIと環境制御を提供する。
  source/competitor: OpenHands
  implementation target: Setup Doctorとインストーラーの復旧操作
  classification: required_for_v1
  blocks_release: yes

- requirement: ローカルLLMランタイム向けにランタイムマニフェストとアダプターマニフェストを提供する。
  source/competitor: LM Studio、Open WebUI、Ollama
  implementation target: Runtime Catalog
  classification: required_for_v1
  blocks_release: yes

- requirement: エージェントセッション、タスク、ワークスペース、ツール呼び出し、差分、コミット、承認、監査ログを第一級の契約として扱う。
  source/competitor: Codex系コーディングエージェントおよびAIDev論文
  implementation target: Agent Runtime Contract
  classification: required_for_v1
  blocks_release: yes

- requirement: 主要なコーディングエージェントすべてとの実運用統合を提供する。
  source/competitor: Codex、Claude Code、Copilot、Cursor、Devin、OpenHands
  implementation target: アダプターパッケージ
  classification: post_v1_scope
  reason: v1.0では汎用のAgent Runtime契約とmock/reference agentが必要である。
  blocks_release: no

- requirement: クラウド同期と複数ユーザー管理を提供する。
  source/competitor: より広いランタイム管理製品群
  implementation target: v1以後のサービス層
  classification: post_v1_scope
  reason: v1.0はデスクトップ優先、ローカル優先、単一ユーザー向けである。
  blocks_release: no

- requirement: v1.0ではローカル単一ユーザーモードだけを提供する。
  source/competitor: AnythingLLM Desktopの位置付け
  implementation target: README、CLAIM、インストーラー、Shell Coreの前提
  classification: known_limitation
  reason: 意図して定めたv1.0の範囲である。
  blocks_release: no
~~~

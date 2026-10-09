# MCP Memory Ledger プロジェクト概要

## 要約

MCP Memory Ledger は、長期記憶・自己スナップショット・反省更新の最小ループを検証するための Rust 製ローカル MCP `stdio` memory demo です。現在の実装は SQLite を永続化基盤としており、完成した製品というより、技術デモ、統合プロトタイプ、研究向け MVP として位置付けるのが適切です。

互換性メモ：現在の Rust crate、binary、script、設定例、および一部の履歴ドキュメントでは、技術識別子として `agent_llm_mm` / `agent-llm-mm` がまだ使われています。公開プロジェクト名は MCP Memory Ledger に統一します。

## 現在のスコープ

- ローカル MCP `stdio` サーバー
- SQLite 永続化
- 設定ファイル駆動の provider 読み込み
- `openai-compatible` / OpenRouter provider
- `ingest_interaction`
- `search_memory` / `get_memory` / `get_reflection_history` / `get_self_model_history` / `get_evidence_relation` / `supersede_memory`
- `build_self_snapshot`
- `decide_with_snapshot`
- `run_reflection`
- `doctor` / `serve` 起動エントリ

## 現在の境界

- M1 は M1.2.7 まで十六切片完了：四種 scoped search/lookup、union、Claim history、self-model audit、evidence-relation runtime、scoped Claim supersede。次は M1.3.0 current-schema structural readback
- `decide_with_snapshot` は `openai-compatible` または OpenRouter provider を利用できますが、返却契約はまだ最小の action string です
- リモート HTTP transport はありません
- `get_evidence_relation` には scoped runtime の第一切片があるが、richer ranking / weighting ではない
- Azure とローカルモデル provider はまだ未実装です。OpenRouter は OpenAI-compatible transport を利用し、明示的な live runner は provider preflight evidence だけを生成します。provider quality、SLA、gateway certification ではありません
- release evidence index、provider certification preflight、packaging preflight、richer memory semantics projection はローカル read-only / preflight 能力です。不足している product evidence を生成せず、provider quality を認証せず、installer を作成せず、より完全な多層 memory モデルの完成も意味しません

## 適した用途

- ローカル AI クライアント統合の検証
- self-agent memory の技術デモ
- Rust + MCP + SQLite の最小構成リファレンス

## ドキュメント運用ルール

- 各タスクの完了後、そのタスクが挙動、能力境界、接続手順、設定、検証コマンド、または協業ルールに影響する場合は、対応するドキュメントを必ず同時に更新します。
- ドキュメント更新を最後にまとめて回すのではなく、可能な限りコード変更と同じタスク内で一緒に収束させます。
- [正式化改善計画](formalization-improvement-plan-2026-08-25.md)は gap と acceptance の対応表であり、第二の実行キューではありません。現在の catch-up 期間は既存の `dev-work -> main` PR で統合し、`main` への merge には fresh CI と明示的な human review が必要です。

## 現在の検証状態

`2026-08-25` 時点で：

- tests は `fast` / `core` / `full` の 3 tiers に分かれ、default core は release tooling を build しません
- `release-tools` feature は release evidence、packaging、provider certification の検証を保持します
- MCP `stdio` は現在 10 ツール。identity / commitment / reflection の durable write path は `run_reflection` のみ
- M0 は閉じ、M1 は M1.2.7 まで完了。次の切片は M1.3.0
- fresh local CI-equivalent gate は format、all-target/all-feature Clippy、full tier、status sync、diff check を通過。remote PR checks は別 gate
- `doctor` は `status = ok` を返します

## 謝辞

このリポジトリは、OpenAI Codex を協調的な開発ツールとして活用しながら、実装・議論・文書整理を進めてきました。このようなワークフローを可能にするツール群と研究エコシステムを提供している OpenAI に感謝します。

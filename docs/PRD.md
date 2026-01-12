# Product Requirements Document (PRD)

**Project Name (仮):** sukkiri
**Version:** 0.1 (Draft)
**Date:** 2026-01-10
**Language:** Rust

## 1. プロジェクト概要 (Overview)

### 1.1 コンセプト

CleanMyMacやBuhoCleanerのような「直感的なシステムクリーニング体験」を、ターミナル（CLI）上で実現する。Rust言語を採用し、既存のGUIツールよりも**高速**で、スクリプトよりも**視覚的かつ安全**なツールを目指す。

### 1.2 ターゲットユーザー

* GUIよりもターミナルを好む開発者・パワーユーザー。
* `node_modules` や `Docker` イメージなど、開発特有のゴミファイルに悩むエンジニア。
* 低スペックなMacを使用していて、軽量なメンテナンスツールを求めているユーザー。

## 2. 機能要件 (Functional Requirements)

### 2.1 コア機能 (MVP: Minimum Viable Product)

| 機能カテゴリ | ID | 機能名 | 詳細・挙動 |
| --- | --- | --- | --- |
| **スキャン** | F-01 | **高速スキャン** | 指定されたカテゴリ（キャッシュ、ログ等）を並列処理でスキャンし、削除可能なファイルサイズを計算する。 |
|  | F-02 | **開発者ジャンク検出** | `node_modules` (一定期間未アクセスのもの)、Xcode `DerivedData`、`Cargo target`、Docker不要イメージを検出対象とする。 |
|  | F-03 | **システムジャンク検出** | ユーザーキャッシュ (`~/Library/Caches`), ログ (`~/Library/Logs`), 破損した設定ファイルを検出。 |
| **操作・UI** | F-04 | **TUIダッシュボード** | スキャン結果をカテゴリ別にリスト表示し、項目ごとの容量を表示する（Ratatui使用）。 |
|  | F-05 | **インタラクティブ選択** | ユーザーが削除したい項目をスペースキーでON/OFF選択できる。 |
|  | F-06 | **詳細プレビュー** | カテゴリを選択した際、右ペインに具体的に削除されるファイルパスや内訳を表示する。 |
| **削除実行** | F-07 | **Dry Run (初期値)** | デフォルトでは「削除される予定のリスト」を表示するのみで、実際の削除は行わない。 |
|  | F-08 | **ゴミ箱への移動** | 安全のため、`rm` で完全削除するのではなく、システムのゴミ箱へ移動させるオプションを標準とする。 |
|  | F-09 | **Force Clean** | ユーザーが明示的にフラグを立てた場合のみ、ゴミ箱を経由せず完全削除を行う。 |

### 2.2 今後の拡張機能 (Future Scope)

* **言語ファイル削除:** アプリ内の不要な `.lproj` フォルダ削除（※署名破損リスクがあるため慎重に実装）。
* **大容量ファイル検索:** 指定サイズ以上のファイルをリストアップ。
* **Cron/Daemonモード:** 定期的にバックグラウンドでスキャンし、通知を送る。

## 3. 技術仕様 (Technical Specifications)

### 3.1 技術スタック

* **言語:** Rust (Edition 2021)
* **アーキテクチャ:** MVCライクな構成 (Model: スキャナロジック, View: TUI描画, Controller: 入力ハンドリング)

### 3.2 推奨ライブラリ (Crates)

| 用途 | ライブラリ | 選定理由 |
| --- | --- | --- |
| **CLI引数解析** | `clap` | デファクトスタンダード。v4系のBuilder APIを使用。 |
| **TUI描画** | `ratatui` | リッチなUI構築に必須。ウィジェットが豊富。 |
| **ターミナル操作** | `crossterm` | バックエンド処理、イベント取得。 |
| **並列処理** | `rayon` / `crossbeam` | ディレクトリ探索の高速化。 |
| **ファイル探索** | `jwalk` or `ignore` | `ripgrep` で使われている爆速ウォーカー。`.gitignore` の考慮も可能。 |
| **システム情報** | `sysinfo` | ディスク総容量やメモリ状況の取得（ヘッダー表示用）。 |
| **ゴミ箱操作** | `trash` | プラットフォームごとのゴミ箱APIを抽象化してくれる。 |
| **ファイルサイズ** | `humansize` | バイト数を "1.2 GB" などの人間に読みやすい形式に変換。 |

### 3.3 データ構造設計案

```rust
// スキャン結果を保持する構造体
struct ScanResult {
    category: CategoryType, // SystemLogs, Xcode, NodeModules, etc.
    total_size: u64,
    items: Vec<ScannedItem>,
    is_selected: bool,
}

enum CategoryType {
    SystemCache,
    UserLogs,
    XcodeDerivedData,
    NodeModules,
    DockerImages,
    // ...
}

struct ScannedItem {
    path: PathBuf,
    size: u64,
    modified_at: SystemTime,
}

```

## 4. 非機能要件 (Non-Functional Requirements)

### 4.1 パフォーマンス

* **スキャン速度:** 100GB程度のホームディレクトリのスキャン（特定ターゲット）を数秒〜10秒以内で完了すること。
* **メモリ使用量:** 大量のファイルパスを保持しても、100MB以下に抑える努力をする（ストリーミング処理など）。

### 4.2 安全性 (Safety)

* **Allowlist (保護リスト):** システム上重要なファイルや、ChromeのCookieファイルなどはハードコードで除外リストに入れる。
* **権限チェック:** フルディスクアクセス権限がない場合、警告を表示してユーザーに設定を促す。

### 4.3 ユーザビリティ

* **キーボード操作:** Vimライク (`j`, `k`) および矢印キーでの移動をサポートする。
* **レスポンシブ:** ターミナルサイズを変更してもレイアウトが崩れず、再描画されること。

## 5. UIデザイン (Wireframe)

### メインダッシュボード画面イメージ

```text
  sukkiri v0.1.0  |  Disk: 450GB / 1TB (45% Used)
 ──────────────────────────────────────────────────────────────────────────
  [Categories]                 |  [Details: Xcode DerivedData]
                               |
  [x] System Caches  1.2 GB    |  Path: ~/Library/Developer/Xcode/DerivedData
  [ ] User Logs      300 MB    |  Status: Safe to delete (Rebuilds on launch)
  [x] Xcode Junk     4.5 GB    |
  [ ] Node Modules   8.2 GB    |  Contains 14 projects' build artifacts:
  [ ] Docker Images  2.1 GB    |   - Project A (2 weeks ago)
                               |   - Project B (1 month ago)
                               |
 ──────────────────────────────────────────────────────────────────────────
  Total Selected: 5.7 GB       |  [Space] Toggle  [Enter] Clean  [q] Quit

```

---

## 6. 開発ロードマップ

### Phase 1: プロトタイピング (Day 1-2)

* `cargo new`
* `ratatui` を導入し、上記のUIの「ガワ（見た目）」だけを作る。
* ダミーデータを表示して、キーボード操作で選択・非選択ができるようにする。

### Phase 2: スキャナー実装 (Day 3-5)

* 実際のファイルシステムを探索するロジックの実装。
* まずは `~/Library/Caches` などの安全な場所から。
* UIとロジックの結合（スキャン中のプログレスバー表示）。

### Phase 3: クリーナー実装 & 安全機構 (Day 6-7)

* `trash` クレートの組み込み。
* Dry Runモードの実装。
* ビルドと手元でのテスト。

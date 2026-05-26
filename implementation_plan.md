# FastMM Editor Phase 8: UI/UX & MD Parsing Implementation Plan (Refined)

本ドキュメントは、FastMMエディタのUI/UXの大幅な進化（メニューバー・ツールバーの導入）および、Markdownファイル読み込み時にリッチテキスト（WYSIWYG）へ正しく変換・保持されない問題の修正に関する実装計画書です。

## 課題の背景と修正方針（Opusレビュー反映）

### 1. MD読み込み・同期時のMarkdown復元バグ (Rust Backend)

- **現状**: `open_file` でファイルをロードした直後やキー入力の同期 (`sync_block`) 時、インライン装飾のみを復元する `ast_to_markdown` が使われていたり、`plain_text` がそのまま上書きされたりして、`#` (見出し) や `-` (リスト) のブロック記号が消失していました。また、同期処理において `current_markdown` は計算されるものの使用されていないデッドコードになっていました。
- **修正方針**:
  - `open_file` で初期データをセットする際、インライン装飾だけの `ast_to_markdown` ではなく、元のファイルからロードしたRAWテキスト (`b.plain_text` または元々のmarkdown情報) をそのまま `BlockState.markdown` に初期保持します。
  - フロントエンドから `BlockSyncRequest` で `block_type` を送るのではなく、Rust側で保持している `BlockState.block_type` を正（Source of Truth）とします。
  - 同期処理 (`process_sync_request`) 内で、`ast_to_markdown(&req.decorations)` によって得られたインラインMarkdownに対して、バックエンド側で保持する `BlockState.block_type` に応じたプレフィックス（例: `#`, `-`, `>`）を前置して `target.markdown` を再構築します。
  - 同期処理内での `derive_block_type` 再計算は不要なため削除します（ブロック型の変更は `apply_format` 経由のみに制限）。

### 2. UI/UX の進化と状態管理 (Svelte Frontend)

- **状態管理のStore化**: `MenuBar` や `Toolbar` からエディタの共通状態（`blocks`, `isDirty`, `nodeOrder` 等）に安全にアクセス・変更するため、SvelteのカスタムStore（`editorStore`）を導入して状態を一元管理します。
- **選択状態（Selection）の維持**: ツールバーボタン（B, I, H1〜H3等）をクリックした際に、contenteditableからフォーカスが失われて選択範囲が消えるのを防ぐため、ボタンの `mousedown` イベントに `preventDefault` を適用します。
- **`seq` 番号の競合解消**: キー同期とフォーマット適用処理での `seq` バグ（stale判定で更新が破棄される）を防ぐため、フロントエンド側で一元管理される単調増加カウンタを導入し、すべてのリクエストで共有します。
- **グラフィカルブロックの制限**: MathBlock、Mermaid等のエディタがアクティブな場合はツールバーの書式ボタンを無効化します。

---

## Proposed Changes (修正内容)

### 1. Backend (Rust)

#### [MODIFY] [editor_state.rs](file:///d:/%E3%82%A2%E3%83%97%E3%83%AA/FastMM/src-tauri/src/editor_state.rs)

- `process_sync_request` 内で、デッドコードになっている `current_markdown` 計算部分を修正し、`target.block_type` に基づいて正しい Markdown プレフィックスを `ast_to_markdown` の結果に前置するように変更。
- 同期時の `derive_block_type` 呼び出しを削除。

#### [MODIFY] [lib.rs](file:///d:/%E3%82%A2%E3%83%97%E3%83%AA/FastMM/src-tauri/src/lib.rs)

- `open_file` 時にブロックを構築する際、Markdownファイルから読み込んだオリジナルのプレフィックス付きテキストが `BlockState.markdown` に保持されるように初期化処理を修正。

---

### 2. Frontend (Svelte)

#### [NEW] [editorStore.js](file:///d:/%E3%82%A2%E3%83%97%E3%83%AA/FastMM/src/lib/editorStore.js)

- `blocks`, `nodeOrder`, `isDirty`, `activeBlockId`, `activeBlockElement`, `monotonicSeq` などを一元管理する Svelte store を新規作成。

#### [MODIFY] [Editor.svelte](file:///d:/%E3%82%A2%E3%83%97%E3%83%AA/FastMM/src/lib/Editor.svelte)

- 状態管理を `editorStore` に移行。
- 画面上部にメニューバー（File, Edit, View, Settingsタブ）と、その下に書式ツールバー（B, I, H1〜H3, List等）を配置。
- ツールバーボタンクリック時に `on:mousedown|preventDefault` を設定して選択状態を維持。
- アクティブなブロックタイプに応じて無効化処理を追加。

#### [MODIFY] [Block.svelte](file:///d:/%E3%82%A2%E3%83%97%E3%83%AA/FastMM/src/lib/Block.svelte)

- `on:focus` イベントにて、contenteditableのDOM要素そのもの（`bind:this` で取得した参照）を `editorStore` の `activeBlockElement` にセットするよう修正。

---

## Verification Plan (検証計画)

### Automated Tests

- `npm run test` または既存のバックエンドテストスイートを実行し、既存のASTテストやエディタステートテストが正常に通ることを確認する。

### Manual Verification

1. **MDファイルの読み込み**: H1見出し、リスト、太字を含むMarkdownファイルを読み込み、リッチテキストとして正しくレンダリングされること。
2. **編集時のフォーマット維持**: 読み込んだH1見出しやリストに対して文字を入力した際、見出しやリストのスタイルがParagraphに戻らず維持されること。
3. **ツールバーフォーマット適用**: 文字を選択してツールバーの「B」ボタンをクリックした際、選択範囲が失われずに太字が適用されること。
4. **保存整合性**: 編集後にファイルを保存し、元のMarkdown構文（`#` や `**` 等）が欠落せずに保存されていること。

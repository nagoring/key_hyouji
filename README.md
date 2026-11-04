# key_hyouji

押したキー・ショートカットを画面にオーバーレイ表示するデスクトップアプリ（Windows / macOS）。
画面収録・配信・プレゼン向け。仕様は [Docs/SPEC.md](Docs/SPEC.md)。

## できること（v1.1）

- どのアプリを使っていてもキー入力を表示（文字は `hello` のようにまとめ、ショートカットは `⌘ ⇧ S` / `Ctrl + Shift + S`、連打は `×3`）
- 背景透過・常に最前面・クリック透過のオーバーレイ
- **設定画面**（トレイ →「設定…」）。変更はすぐに反映・自動保存
  - 一般：表示モード、表示時間、フェード時間、同時表示数、文字をまとめる間隔、修飾キーの表記
  - 外観：テーマ（ダーク / ライト / キーキャップ / ミニマル）、フォント、サイズ、太さ、色、不透明度、角丸、枠線、影、アニメーション、プレビュー
  - 位置：9 か所のプリセット、ドラッグ配置、表示モニター（メイン / カーソル追従 / アクティブウィンドウ追従 / 固定）、余白
  - マウス：クリックの表示（左 / 右 / 中、ダブルクリックは ×2。既定でオン、トレイからも切り替え可）、クリック位置のリング
  - 配信用モード：背景を単色（グリーンバックなど）にして OBS のクロマキーで抜ける
  - 情報：設定のエクスポート / インポート（JSON）
  - プライバシー：パスワード欄では表示しない（🔒 で通知）、日本語入力（IME）中の変換前の文字を隠す、除外アプリ
  - ホットキー：一時停止キーの変更（既定 `Ctrl+Alt+Shift+K` / `⌃⌥⇧K`）
- ログイン時の自動起動（Windows はスタートアップフォルダ、macOS は LaunchAgent。レジストリ不使用）
- 日本語 / 英語（OS の言語に自動で合わせる。設定で切り替え可）
- macOS：初回起動時に「入力監視」の許可を案内する画面
- キー入力は保存・送信しない

## 開発

必要なもの: Node.js 20+、Rust（stable）、各 OS の Tauri 前提条件
（Windows: WebView2 / MSVC Build Tools、macOS: Xcode Command Line Tools）

```bash
npm install
npm run tauri dev      # 開発起動
npm run tauri build    # Windows: target/release/key_hyouji.exe（単体）/ macOS: .dmg
cargo test -p key_core # コアロジックのテスト（どの OS でも可）
```

### Windows: レジストリを使いません

インストーラーは無く、`key_hyouji.exe` を置いて実行するだけです（アプリもレジストリを読み書きしません）。
削除は exe と、`%APPDATA%\com.nagoring.keyhyouji\`・`%LOCALAPPDATA%\com.nagoring.keyhyouji\` を消すだけです。
WebView2 ランタイムが必要です（Windows 11 は標準搭載）。

### macOS の権限

初回起動時に「入力監視」の許可を求めます。
システム設定 › プライバシーとセキュリティ › 入力監視 で key_hyouji をオンにしてください
（トレイメニューの「入力監視の設定を開く…」からも開けます）。許可されると自動で表示が始まります。
反映されない場合はアプリを再起動してください。

## リリース・コード署名

[Docs/RELEASE.md](Docs/RELEASE.md) を参照。`v*` タグを push すると CI が下書きの Release を作ります。

## 構成

```
crates/key_core/   OS 非依存のロジック（集約・表記・設定）と OS 別キーフック
  src/hook/windows.rs   WH_KEYBOARD_LL
  src/hook/macos.rs     CGEventTap（listen-only）
src-tauri/         Tauri アプリ（トレイ、オーバーレイ配置、一時停止）
src/               オーバーレイ描画（TypeScript）
```

設定ファイル: `<アプリ設定ディレクトリ>/settings.json`
（Windows: `%APPDATA%\com.nagoring.keyhyouji\`、macOS: `~/Library/Application Support/com.nagoring.keyhyouji/`）

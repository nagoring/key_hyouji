# key_hyouji

押したキー・ショートカットを画面にオーバーレイ表示するデスクトップアプリ（Windows / macOS）。
画面収録・配信・プレゼン向け。仕様は [Docs/SPEC.md](Docs/SPEC.md)。

## v0.1 でできること

- どのアプリを使っていてもキー入力を表示（文字は `hello` のようにまとめ、ショートカットは `⌘ ⇧ S` / `Ctrl + Shift + S`、連打は `×3`）
- 2 秒でフェードアウト、最大 3 件
- 背景透過・常に最前面・クリック透過のオーバーレイ
- トレイ / メニューバーから：一時停止、表示モード（すべて / ショートカットのみ）、表示位置（9 か所）、配置モード（ドラッグで移動）、終了
- 一時停止ホットキー: `Ctrl+Alt+Shift+K`（Mac: `⌃⌥⇧K`）
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

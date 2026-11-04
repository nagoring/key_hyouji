# キーストローク表示アプリ 仕様書（key_hyouji）

- 版: 0.1（ドラフト）
- 作成日: 2026-11-04
- 対象OS: Windows 10/11（x64 / ARM64）、macOS 12 Monterey 以降（Apple Silicon / Intel）

---

## 1. 目的・概要

画面収録・ライブ配信・プレゼン・操作説明動画などで、**押されたキー（およびショートカット）を画面上にオーバーレイ表示**するデスクトップアプリ。

### 想定ユーザー / ユースケース
| ユースケース | 求められること |
|---|---|
| チュートリアル動画の収録 | ショートカットが視聴者に読める大きさで表示される |
| ライブ配信（OBS等） | 配信画面に重ねられる、背景透過 / クロマキー対応 |
| プレゼン・ライブコーディング | 邪魔にならない位置・自動フェードアウト |
| 社内操作説明 | インストールが簡単、設定が少ない |

### 非目標（スコープ外）
- キー入力の記録・保存・送信（キーロガー的用途）
- キーリマップ・マクロ機能
- Linux 対応（将来検討）

---

## 2. 機能要件

### 2.1 キー表示（コア機能）
| ID | 要件 | 優先度 |
|---|---|---|
| F-01 | グローバル（どのアプリがアクティブでも）キー押下を検知し表示する | 必須 |
| F-02 | 修飾キー付きの組み合わせを1つのまとまりで表示（例: `Ctrl + Shift + S` / `⌘ ⇧ S`） | 必須 |
| F-03 | 連続した通常文字は1つの吹き出しにまとめて表示（例: `hello`）。一定時間（既定 1.0秒）入力が無ければ区切る | 必須 |
| F-04 | 同じキー/組み合わせの連打は回数表示（例: `⌘ Z ×3`） | 必須 |
| F-05 | 表示は一定時間後（既定 2.0秒）にフェードアウト | 必須 |
| F-06 | 同時表示数の上限（既定 3件）。超えたら古い順に消す | 必須 |
| F-07 | 特殊キーは記号/名称で表示（Enter `⏎`、Backspace `⌫`、Tab `⇥`、Esc、矢印 `← ↑ → ↓`、F1〜F12、Space `␣` 等） | 必須 |
| F-08 | OSに合わせた修飾キー表記（Win: `Ctrl` `Alt` `Shift` `Win` / Mac: `⌃` `⌥` `⇧` `⌘`）。設定で文字表記にも切替可 | 必須 |
| F-09 | 表示モード切替：①すべてのキー ②修飾キー付きのみ（ショートカットのみ） | 必須 |
| F-10 | マウスクリック表示（左/右/中クリック、任意でカーソル位置にリング表示） | 任意（v1.1） |
| F-11 | IME入力中（日本語変換中）は変換確定前のキーをまとめるか、非表示にするかを選択可 | 推奨 |
| F-12 | 修飾キー単体押下は表示しない（既定）。設定でON可 | 推奨 |

### 2.2 オーバーレイウィンドウ
| ID | 要件 |
|---|---|
| W-01 | 枠なし・背景透過・常に最前面 |
| W-02 | クリック透過（マウス操作を背後のアプリに通す） |
| W-03 | タスクバー / Dock / Alt+Tab / Mission Control に表示しない |
| W-04 | 表示位置: 画面の9分割プリセット（既定: 下中央）＋ドラッグでの自由配置（配置モード中のみクリック透過を解除） |
| W-05 | マルチモニター対応：表示モニターを選択（固定 / マウスカーソルのあるモニター / アクティブウィンドウのモニター） |
| W-06 | フルスクリーンアプリ上でも表示（macOS: 全Space・フルスクリーン上に表示） |
| W-07 | 高DPI（Windows スケーリング / Retina）で鮮明に描画 |

### 2.3 外観カスタマイズ
- フォント（種類・サイズ・太さ）、文字色、背景色、背景の不透明度、角丸、枠線、影
- テーマプリセット：ダーク（既定）/ ライト / キーキャップ風 / ミニマル
- アニメーション：フェード / スライド / なし、表示時間・フェード時間
- 配信用：背景を単色（グリーンバック等）にするモード

### 2.4 操作・常駐
| ID | 要件 |
|---|---|
| C-01 | システムトレイ（Win）/ メニューバー（Mac）に常駐。メニュー: 表示ON/OFF、一時停止、設定、終了 |
| C-02 | グローバルホットキーで表示の一時停止/再開（既定: `Ctrl+Alt+Shift+K` / `⌃⌥⇧K`、変更可） |
| C-03 | 設定ウィンドウ（GUI）。変更は即時プレビューに反映 |
| C-04 | OS起動時に自動起動（ON/OFF） |
| C-05 | 設定のインポート/エクスポート（JSON） |
| C-06 | UI言語: 日本語 / 英語（OS言語に追従、手動切替可） |

### 2.5 プライバシー・セキュリティ
| ID | 要件 |
|---|---|
| P-01 | **キー入力をディスク・ネットワークへ一切保存/送信しない**（メモリ上の表示用バッファのみ、フェード後に破棄） |
| P-02 | パスワード入力欄では表示しない（macOS: Secure Input 有効時は自動的にイベントが来ない＋検知して「🔒」表示。Windows: パスワードボックスの判定をUI Automationで試み、判定できた場合は非表示） |
| P-03 | 除外アプリ指定（例: パスワードマネージャー、ターミナル）。該当アプリがフォアグラウンドの間は非表示 |
| P-04 | テレメトリなし。アップデート確認のみ任意でON |
| P-05 | 一時停止中はトレイアイコンを変化させ、状態を明示 |

---

## 3. 非機能要件

| 項目 | 目標 |
|---|---|
| 表示遅延 | キー押下から表示まで 50ms 以内（体感で遅れない） |
| CPU使用率 | アイドル時 0.5% 未満、連打時 5% 未満 |
| メモリ | 常駐時 100MB 未満 |
| 配布サイズ | インストーラー 15MB 未満 |
| 安定性 | フック異常時は自動で再登録。アプリが落ちても入力に影響しない |
| 起動時間 | 2秒以内 |

---

## 4. 技術構成

### 4.1 採用技術
| レイヤー | 技術 | 理由 |
|---|---|---|
| アプリ基盤 | **Tauri 2** | 軽量（Electron比でサイズ・メモリ小）、Win/Mac両対応、透過ウィンドウ対応 |
| バックエンド | **Rust** | OSネイティブAPI（キーフック）を直接扱える |
| フロントエンド | TypeScript + React（または Svelte）+ Vite | オーバーレイ描画・設定画面 |
| 設定保存 | JSON（`tauri-plugin-store`） | |
| 自動起動 | `tauri-plugin-autostart` | |
| ホットキー | `tauri-plugin-global-shortcut` | |
| 更新 | `tauri-plugin-updater`（任意ON） | |

### 4.2 キー入力取得（OS別）
| OS | 方式 | 備考 |
|---|---|---|
| Windows | `SetWindowsHookExW(WH_KEYBOARD_LL)`（低レベルキーボードフック）＋ `WH_MOUSE_LL`（マウス） | `windows` クレート使用。専用スレッドでメッセージループを回す。管理者権限で動くアプリ上ではイベントが取れないため、必要に応じて「管理者として実行」オプションを用意 |
| macOS | `CGEventTapCreate`（listen-only, `kCGEventKeyDown/Up/FlagsChanged`） | `core-graphics` クレート使用。**入力監視（Input Monitoring）権限**が必要。listen-only で入力は改変しない |

- キーコード → 表示名の変換は**キーボードレイアウトを考慮**（JIS/US配列）。
  - Windows: `ToUnicodeEx` / `GetKeyNameTextW`
  - macOS: `UCKeyTranslate`（TIS API）
- 抽象化: `trait KeySource { fn start(tx: Sender<KeyEvent>); fn stop(); }` を OS別に実装。

### 4.3 アーキテクチャ
```
[OSフック(Rust, 専用スレッド)]
      │ KeyEvent { code, kind(down/up), modifiers, timestamp }
      ▼
[イベント処理(Rust)]
  - 修飾キー状態管理
  - 除外アプリ / パスワード欄 / 一時停止 判定
  - 表示用トークン化（"⌘ ⇧ S"、"hello"、連打カウント）
      │ Tauri event "keystroke"
      ▼
[オーバーレイWebView(TS)]  ← 描画・アニメーション
[設定WebView(TS)]          ← 設定変更 → Rust側へ invoke → store保存 → オーバーレイへ反映
[トレイ(Rust)]
```
- キーの集約ロジック（F-03, F-04）はRust側で行い、フロントには「表示すべき文字列」のみ渡す（フロントの負荷・情報量を最小化）。

### 4.4 ウィンドウ実装の注意点
| 項目 | Windows | macOS |
|---|---|---|
| クリック透過 | `WS_EX_TRANSPARENT \| WS_EX_LAYERED`（Tauri `set_ignore_cursor_events(true)`） | 同API（`ignoresMouseEvents`） |
| 最前面 | `HWND_TOPMOST` | `NSWindowLevel` を `screenSaver` 相当に |
| タスク切替から除外 | `WS_EX_TOOLWINDOW` | `NSApplicationActivationPolicyAccessory`（Dockアイコン非表示） |
| 全Space表示 | — | `canJoinAllSpaces` + `fullScreenAuxiliary` |

---

## 5. 権限・初回起動フロー

### macOS
1. 初回起動時にオンボーディング画面を表示し、「入力監視」権限が必要な理由を説明。
2. `IOHIDRequestAccess` / `CGPreflightListenEventAccess` で権限確認 → 未許可なら「システム設定を開く」ボタン（`x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent`）。
3. 許可後はアプリ再起動を促す（macOSの仕様上必要）。
4. 権限が取り消された場合も検知して再案内。

### Windows
- 特別な権限は不要。SmartScreen 警告回避のためコード署名を行う。

---

## 6. 設定項目（既定値）

```json
{
  "enabled": true,
  "displayMode": "all",              // "all" | "shortcutsOnly"
  "showModifierOnly": false,
  "mergeTypingMs": 1000,
  "displayDurationMs": 2000,
  "fadeDurationMs": 300,
  "maxItems": 3,
  "position": { "preset": "bottom-center", "offsetX": 0, "offsetY": 80 },
  "monitor": "cursor",               // "primary" | "cursor" | "active" | <monitorId>
  "modifierStyle": "symbol",         // "symbol" | "text"
  "imeMode": "hide",                 // "hide" | "show"
  "theme": {
    "preset": "dark",
    "fontFamily": "system-ui",
    "fontSize": 32,
    "textColor": "#FFFFFF",
    "bgColor": "#000000",
    "bgOpacity": 0.7,
    "borderRadius": 12,
    "animation": "fade"
  },
  "mouse": { "showClicks": false, "showRing": false },
  "excludedApps": [],
  "pauseHotkey": "CmdOrCtrl+Alt+Shift+K",
  "autoStart": false,
  "checkUpdates": false,
  "language": "auto"
}
```

保存場所:
- Windows: `%APPDATA%\key_hyouji\settings.json`
- macOS: `~/Library/Application Support/key_hyouji/settings.json`

---

## 7. 画面構成

1. **オーバーレイ**: キー吹き出しのみ。配置モード時はガイド枠を表示しドラッグ可能。
2. **設定ウィンドウ**（タブ構成）
   - 一般：表示モード、表示時間、自動起動、言語
   - 外観：テーマ、フォント、色、アニメーション、ライブプレビュー
   - 位置：プリセット、モニター選択、「配置モード」ボタン
   - プライバシー：除外アプリ、パスワード欄非表示、IME
   - ホットキー
   - 情報：バージョン、ライセンス、更新確認
3. **オンボーディング**（macOS権限案内）
4. **トレイメニュー**

---

## 8. 配布・ビルド

| OS | 形式 | 署名 |
|---|---|---|
| Windows | `.msi` / `.exe`（NSIS） | Authenticode コード署名 |
| macOS | `.dmg`（Universal Binary: arm64 + x86_64） | Developer ID 署名 + Notarization |

- CI: GitHub Actions（`tauri-action`）で Windows / macOS をマトリクスビルド、タグ push で Release 作成。

---

## 9. テスト方針
- **ユニットテスト（Rust）**: キー集約ロジック（文字結合・連打カウント・修飾キー表記・区切り時間）
- **統合テスト**: 擬似KeyEventを流してフロントへのイベント出力を検証
- **手動テスト項目**:
  - JIS/US配列、日本語IME ON/OFF
  - マルチモニター・異なるDPI混在
  - フルスクリーンアプリ（ゲーム・動画・PowerPoint）上での表示
  - OBSでのウィンドウキャプチャ / 画面キャプチャ
  - パスワード欄・除外アプリでの非表示
  - macOS 権限未許可 → 許可 → 取り消し
  - スリープ復帰後のフック継続

---

## 10. マイルストーン

| フェーズ | 内容 |
|---|---|
| v0.1 (MVP) | F-01〜F-08、オーバーレイ（W-01〜W-04）、トレイ、一時停止、P-01 |
| v0.2 | 設定GUI、テーマ、マルチモニター、除外アプリ、macOSオンボーディング |
| v1.0 | パスワード欄対策、IME対応、署名済みインストーラー、自動起動、多言語 |
| v1.1 | マウスクリック表示、配信用モード、設定インポート/エクスポート |

---

## 11. 未決事項
- フロントエンドフレームワーク（React / Svelte）の最終決定
- アプリ名・アイコン（仮: key_hyouji）
- ライセンス（MIT 想定）
- 自動アップデートの配信元（GitHub Releases 想定）

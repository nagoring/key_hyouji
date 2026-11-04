# リリース手順

## 1. バージョンを上げる

次の 4 か所を同じ値にする。

- `package.json` の `version`
- `src-tauri/tauri.conf.json` の `version`
- `src-tauri/Cargo.toml` / `crates/key_core/Cargo.toml` の `version`

## 2. タグを push する

```bash
git tag v1.0.0
git push origin v1.0.0
```

GitHub Actions がビルドし、**下書きの Release** に次のファイルを添付する。内容を確認して公開する。

- `key_hyouji-<version>-windows-x64.exe`（ポータブル版。レジストリを使わない）
- `key_hyouji-<version>-macos-universal.dmg`（Apple Silicon / Intel 両対応）

## 3. コード署名（任意・推奨）

リポジトリの Settings › Secrets and variables › Actions に次の Secret を登録すると、CI が自動で署名する。
未登録なら署名なしでビルドされる（初回起動時に SmartScreen / Gatekeeper の警告が出る）。

### Windows（Authenticode）

| Secret | 内容 |
|---|---|
| `WINDOWS_CERTIFICATE` | コード署名証明書（.pfx）を Base64 にしたもの |
| `WINDOWS_CERTIFICATE_PASSWORD` | .pfx のパスワード |

```bash
base64 -w0 cert.pfx   # macOS は: base64 -i cert.pfx
```

### macOS（Developer ID 署名 + 公証）

| Secret | 内容 |
|---|---|
| `APPLE_CERTIFICATE` | 「Developer ID Application」証明書を書き出した .p12 の Base64 |
| `APPLE_CERTIFICATE_PASSWORD` | .p12 のパスワード |
| `APPLE_SIGNING_IDENTITY` | 例: `Developer ID Application: Your Name (TEAMID)` |
| `APPLE_ID` | 公証に使う Apple ID |
| `APPLE_PASSWORD` | Apple ID の App 用パスワード |
| `APPLE_TEAM_ID` | チーム ID |

公証済みでも、キー入力の表示には利用者が「入力監視」を許可する必要がある（アプリ内で案内される）。

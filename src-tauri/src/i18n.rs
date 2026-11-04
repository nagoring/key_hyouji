//! Rust 側で表示する文言（トレイ・ウィンドウタイトル・エラー）の日本語 / 英語。
//! 画面内の文言はフロントエンドの `src/i18n.ts` にある。

use key_core::config::{Language, Preset};

pub struct Strings {
    pub settings: &'static str,
    pub pause: &'static str,
    pub display_mode: &'static str,
    pub mode_all: &'static str,
    pub mode_shortcuts: &'static str,
    pub position: &'static str,
    pub arrange: &'static str,
    pub onboarding: &'static str,
    pub quit: &'static str,
    pub tooltip_paused: &'static str,
    pub tooltip_need_permission: &'static str,
    pub tooltip_error: &'static str,
    pub settings_title: &'static str,
    pub onboarding_title: &'static str,
    pub err_hotkey_parse: &'static str,
    pub err_hotkey_register: &'static str,
    pub err_autostart: &'static str,
    pub err_import: &'static str,
    pub err_export: &'static str,
    pub clicks: [&'static str; 3],
    presets: [&'static str; 9],
    paren: (&'static str, &'static str),
}

impl Strings {
    pub fn preset(&self, p: Preset) -> &'static str {
        let i = Preset::ALL.iter().position(|x| *x == p).unwrap_or(7);
        self.presets[i]
    }

    /// 言語に合った括弧で囲む（日本語は全角）
    pub fn with_paren(&self, text: &str, inner: &str) -> String {
        format!("{text}{}{inner}{}", self.paren.0, self.paren.1)
    }
}

const JA: Strings = Strings {
    settings: "設定…",
    pause: "一時停止",
    display_mode: "表示モード",
    mode_all: "すべてのキー",
    mode_shortcuts: "ショートカットのみ",
    position: "表示位置",
    arrange: "配置モード（ドラッグで移動）",
    onboarding: "入力監視の許可について…",
    quit: "終了",
    tooltip_paused: "一時停止中",
    tooltip_need_permission: "入力監視の許可が必要",
    tooltip_error: "エラー",
    settings_title: "key_hyouji 設定",
    onboarding_title: "key_hyouji へようこそ",
    err_hotkey_parse: "ホットキーを解釈できません",
    err_hotkey_register: "ホットキーを登録できません（他のアプリが使用中の可能性）",
    err_autostart: "自動起動を設定できません",
    err_import: "設定ファイルを読み込めません",
    err_export: "設定ファイルを保存できません",
    clicks: ["クリック", "右クリック", "中クリック"],
    paren: ("（", "）"),
    presets: ["左上", "上中央", "右上", "左中央", "中央", "右中央", "左下", "下中央", "右下"],
};

const EN: Strings = Strings {
    settings: "Settings…",
    pause: "Pause",
    display_mode: "Display mode",
    mode_all: "All keys",
    mode_shortcuts: "Shortcuts only",
    position: "Position",
    arrange: "Arrange mode (drag to move)",
    onboarding: "Input Monitoring permission…",
    quit: "Quit",
    tooltip_paused: "paused",
    tooltip_need_permission: "Input Monitoring permission required",
    tooltip_error: "error",
    settings_title: "key_hyouji Settings",
    onboarding_title: "Welcome to key_hyouji",
    err_hotkey_parse: "Invalid hotkey",
    err_hotkey_register: "Could not register the hotkey (it may be used by another app)",
    err_autostart: "Could not configure launch at login",
    err_import: "Could not read the settings file",
    err_export: "Could not save the settings file",
    clicks: ["Click", "Right Click", "Middle Click"],
    paren: (" (", ")"),
    presets: [
        "Top left", "Top center", "Top right", "Middle left", "Center", "Middle right", "Bottom left",
        "Bottom center", "Bottom right",
    ],
};

pub fn strings(lang: Language) -> &'static Strings {
    match lang {
        Language::En => &EN,
        _ => &JA,
    }
}

//! Stock Veritas 0.2.57 config adapter. Never reloads or edits a running plugin.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PluginOptions {
    pub locale: String,
    pub streamer_mode: bool,
    pub streamer_message: String,
    pub widget_opacity: f64,
    pub pie_opacity: f64,
    pub auto_show_hide: bool,
    pub theme_mode: String,
    pub legend_text_style: String,
    pub font_sizes: BTreeMap<String, f64>,
    pub suppress_native: bool,
}
impl Default for PluginOptions {
    fn default() -> Self {
        Self {
            locale: "ru".into(),
            streamer_mode: false,
            streamer_message: "Nous".into(),
            widget_opacity: 0.3,
            pie_opacity: 0.05,
            auto_show_hide: false,
            theme_mode: "Dark".into(),
            legend_text_style: "Body".into(),
            font_sizes: [
                ("Small", 12.0),
                ("Body", 14.0),
                ("Button", 14.0),
                ("Heading", 20.0),
                ("Monospace", 14.0),
            ]
            .into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect(),
            suppress_native: true,
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginConfigView {
    pub directory: String,
    pub options: PluginOptions,
    pub requires_restart: bool,
    pub warning: &'static str,
}
pub const WARNING: &str = "Изменения Veritas читаются при запуске игры. Нужен перезапуск. Официальная DLL не имеет полностью скрытого режима: возможны её уведомления и встроенные Ctrl+M / Ctrl+H. Скрытый сброс AV и внутренний журнал не передаются по Socket.IO.";

const STATE: &str = r#"(show_menu:false,show_changelog:false,show_help:false,show_settings:false,show_console:false,show_damage_distribution:false,show_damage_type_breakdown:false,show_damage_bars:false,show_character_damage:false,show_real_time_damage:false,show_enemy_stats:false,show_battle_metrics:false,hide_ui:true,graph_x_unit:Turn,damage_bar_value:Damage,damage_breakdown_scope:Team,damage_breakdown_chart:Pie,damage_breakdown_character_index:0,show_damage_breakdown_table:false,show_damage_breakdown_legend:false,show_character_legend:false,auto_save_battle_data:false,show_export_window:false,show_updater_window:false,custom_export_path:None,auto_create_date_folders:true)"#;

// Mirrors the serde schema in stock Veritas 0.2.57 ui/app.rs. Keep unknown
// fields in the original text, but require every field the stock loader needs.
#[derive(Deserialize)]
#[allow(dead_code)]
struct StockState {
    show_menu: bool,
    show_changelog: bool,
    show_help: bool,
    show_settings: bool,
    show_console: bool,
    show_damage_distribution: bool,
    #[serde(default)]
    show_damage_type_breakdown: bool,
    show_damage_bars: bool,
    #[serde(default)]
    show_character_damage: bool,
    show_real_time_damage: bool,
    show_enemy_stats: bool,
    show_battle_metrics: bool,
    hide_ui: bool,
    graph_x_unit: GraphUnit,
    #[serde(default)]
    damage_bar_value: DamageBarValue,
    #[serde(default)]
    damage_breakdown_scope: DamageBreakdownScope,
    #[serde(default)]
    damage_breakdown_chart: DamageBreakdownChart,
    #[serde(default)]
    damage_breakdown_character_index: usize,
    #[serde(default)]
    show_damage_breakdown_table: bool,
    #[serde(default)]
    show_damage_breakdown_legend: bool,
    show_character_legend: bool,
    auto_save_battle_data: bool,
    show_export_window: bool,
    show_updater_window: bool,
    custom_export_path: Option<String>,
    auto_create_date_folders: bool,
}
#[derive(Deserialize)]
enum GraphUnit {
    Turn,
    ActionValue,
}
#[derive(Default, Deserialize)]
enum DamageBarValue {
    #[default]
    Damage,
    Dpav,
}
#[derive(Default, Deserialize)]
enum DamageBreakdownScope {
    #[default]
    Team,
    Character,
}
#[derive(Default, Deserialize)]
enum DamageBreakdownChart {
    #[default]
    Pie,
    Bar,
}

fn validate_theme(theme: &Value) -> Result<(), String> {
    // egui_colors 0.13.0 tokens.rs: Theme = [ThemeColor; 12], with exactly
    // these unit variants and the externally tagged Custom([u8; 3]) variant.
    const TOKENS: &[&str] = &[
        "Gray", "EguiBlue", "Tomato", "Red", "Ruby", "Crimson", "Pink", "Plum", "Purple", "Violet",
        "Iris", "Indigo", "Blue", "Cyan", "Teal", "Jade", "Green", "Grass", "Brown", "Bronze",
        "Gold", "Orange",
    ];
    let valid = theme.as_array().is_some_and(|colors| {
        colors.len() == 12
            && colors.iter().all(|color| {
                color.as_str().is_some_and(|token| TOKENS.contains(&token))
                    || color.as_object().is_some_and(|object| {
                        object.len() == 1
                            && object.get("Custom").is_some_and(|rgb| {
                                serde_json::from_value::<[u8; 3]>(rgb.clone()).is_ok()
                            })
                    })
            })
    });
    if valid {
        Ok(())
    } else {
        Err("Палитра Veritas требует 12 известных цветовых токенов или Custom с тремя целыми RGB от 0 до 255".into())
    }
}

fn validate_text_style(style: &str) -> Result<(), String> {
    // A JSON string can encode only these unit variants of egui::TextStyle.
    // Name(Arc<str>) requires a tagged object and cannot be a JSON map key.
    if ["Small", "Body", "Monospace", "Button", "Heading"].contains(&style) {
        Ok(())
    } else {
        Err(format!("Неизвестный стиль текста Veritas: {style}"))
    }
}

/// Mask comments and strings without changing byte offsets used for RON edits.
fn state_code(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut code = bytes.to_vec();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        if bytes[i..].starts_with(b"//") {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if bytes[i..].starts_with(b"/*") {
            let mut depth = 1;
            i += 2;
            while i < bytes.len() && depth > 0 {
                if bytes[i..].starts_with(b"/*") {
                    depth += 1;
                    i += 2;
                } else if bytes[i..].starts_with(b"*/") {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
        } else if bytes[i] == b'"' {
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(bytes.len());
                } else if bytes[i] == b'"' {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
        } else if bytes[i] == b'r' {
            let mut quote = i + 1;
            while quote < bytes.len() && bytes[quote] == b'#' {
                quote += 1;
            }
            if bytes.get(quote) != Some(&b'"') {
                i += 1;
                continue;
            }
            let hashes = quote - i - 1;
            i = quote + 1;
            while i < bytes.len() {
                if bytes[i] == b'"'
                    && bytes
                        .get(i + 1..i + 1 + hashes)
                        .is_some_and(|tail| tail.iter().all(|b| *b == b'#'))
                {
                    i += 1 + hashes;
                    break;
                }
                i += 1;
            }
        } else {
            i += 1;
            continue;
        }
        code[start..i].fill(b' ');
    }
    code
}

fn read_json(path: &Path) -> Result<Value, String> {
    match std::fs::read(path) {
        Ok(data) => serde_json::from_slice(&data).map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(e.to_string()),
    }
}
pub fn atomic_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    atomic(
        path,
        &serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?,
    )
}
pub fn atomic(path: &Path, data: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let temporary = path.with_extension("nous-tmp");
    std::fs::write(&temporary, data)
        .and_then(|_| std::fs::rename(&temporary, path))
        .map_err(|e| format!("{}: {e}", path.display()))
}
fn preserve(path: &Path) -> Result<(), String> {
    let backup = path.with_extension("nous-original");
    if path.exists() && !backup.exists() {
        std::fs::copy(path, backup).map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub fn read(root: &Path) -> Result<PluginOptions, String> {
    let config = read_json(&root.join("config/config.json"))?;
    let mut out = PluginOptions::default();
    macro_rules! string {
        ($field:ident,$key:literal) => {
            if let Some(v) = config[$key].as_str() {
                out.$field = v.into();
            }
        };
    }
    macro_rules! boolean {
        ($field:ident,$key:literal) => {
            if let Some(v) = config[$key].as_bool() {
                out.$field = v;
            }
        };
    }
    string!(locale, "locale");
    string!(streamer_message, "streamer_msg");
    string!(theme_mode, "theme_mode");
    string!(legend_text_style, "legend_text_style");
    boolean!(streamer_mode, "streamer_mode");
    boolean!(auto_show_hide, "auto_showhide_ui");
    if let Some(v) = config["widget_opacity"].as_f64() {
        out.widget_opacity = v;
    }
    if let Some(v) = config["pie_chart_opacity"].as_f64() {
        out.pie_opacity = v;
    }
    if let Some(fonts) = config["font_sizes"].as_object() {
        for (k, v) in fonts {
            if let Some(size) = v["size"].as_f64() {
                out.font_sizes.insert(k.clone(), size);
            }
        }
    }
    let state_path = root.join("data/state");
    if state_path.exists() {
        let state = std::fs::read_to_string(state_path).map_err(|e| e.to_string())?;
        out.suppress_native = ron::from_str::<StockState>(&state)
            .map_err(|e| format!("state RON: {e}"))?
            .hide_ui;
    }
    Ok(out)
}

/// Changes boolean fields outside quoted strings, preserving RON enums and
/// unknown fields exactly. The complete text is parsed before and after editing.
fn patch_state(text: &str, hide: bool) -> Result<String, String> {
    ron::from_str::<StockState>(text).map_err(|e| format!("state RON: {e}"))?;
    let fields = [
        "hide_ui",
        "show_menu",
        "show_changelog",
        "show_help",
        "show_settings",
        "show_console",
        "show_damage_distribution",
        "show_damage_type_breakdown",
        "show_damage_bars",
        "show_character_damage",
        "show_real_time_damage",
        "show_enemy_stats",
        "show_battle_metrics",
        "show_character_legend",
        "show_export_window",
        "show_updater_window",
    ];
    let pattern = regex::Regex::new(r"(?P<key>[a-z_]+)\s*:\s*(?P<value>true|false)\b").unwrap();
    let code = state_code(text);
    let mut depths = vec![0; code.len()];
    let mut depth = 0;
    for (i, byte) in code.iter().enumerate() {
        depths[i] = depth;
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            _ => {}
        }
    }
    let code = String::from_utf8(code).expect("mask preserves UTF-8");
    let mut result = text.to_string();
    for capture in pattern
        .captures_iter(&code)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        let whole = capture.get(0).unwrap();
        let key = &capture["key"];
        if depths[whole.start()] == 1 && fields.contains(&key) && (hide || key == "hide_ui") {
            let value = if key == "hide_ui" && hide {
                "true"
            } else {
                "false"
            };
            let original = capture.name("value").unwrap();
            result.replace_range(original.range(), value);
        }
    }
    ron::from_str::<StockState>(&result).map_err(|e| e.to_string())?;
    Ok(result)
}
pub fn write(
    root: &Path,
    update: &Path,
    version: &str,
    options: &PluginOptions,
) -> Result<(), String> {
    validate_text_style(&options.legend_text_style)?;
    for style in options.font_sizes.keys() {
        validate_text_style(style)?;
    }
    if ![
        "en", "fr", "es", "de", "it", "ja", "pt", "nl", "ru", "vi", "zh", "pl",
    ]
    .contains(&options.locale.as_str())
        || !["Dark", "Light"].contains(&options.theme_mode.as_str())
    {
        return Err("Неизвестный язык или режим темы Veritas".into());
    }
    if ![options.widget_opacity, options.pie_opacity]
        .iter()
        .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
        || options
            .font_sizes
            .values()
            .any(|v| !v.is_finite() || !(8.0..=48.0).contains(v))
    {
        return Err("Недопустимый размер или прозрачность".into());
    }
    let config_path = root.join("config/config.json");
    let state_path = root.join("data/state");
    let mut config = read_json(&config_path)?;
    if !config.is_object() {
        return Err("Конфиг Veritas должен быть JSON-объектом".into());
    }
    let state = match std::fs::read_to_string(&state_path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => STATE.into(),
        Err(e) => return Err(e.to_string()),
    };
    let state = patch_state(&state, options.suppress_native)?;
    config["locale"] = json!(options.locale);
    config["streamer_mode"] = json!(options.streamer_mode && !options.suppress_native);
    config["streamer_msg"] = json!(options.streamer_message);
    config["widget_opacity"] = json!(options.widget_opacity);
    config["pie_chart_opacity"] = json!(options.pie_opacity);
    config["auto_showhide_ui"] = json!(options.auto_show_hide && !options.suppress_native);
    config["theme_mode"] = json!(options.theme_mode);
    config["legend_text_style"] = json!(options.legend_text_style);
    if !config["font_sizes"].is_object() {
        config["font_sizes"] = json!({});
    }
    for (k, v) in &options.font_sizes {
        config["font_sizes"][k] = json!({"size":v,"family":"Proportional"});
    }
    // Validate preserved stock fields too; otherwise a malformed existing value
    // could make the plugin discard the entire newly written config.
    if let Some(theme) = config.get("theme") {
        validate_theme(theme)?;
    }
    for (style, font) in config["font_sizes"].as_object().unwrap() {
        validate_text_style(style)?;
        let size = font["size"]
            .as_f64()
            .ok_or("Некорректный размер шрифта Veritas")?;
        if !size.is_finite() || !(8.0..=48.0).contains(&size) {
            return Err("Недопустимый размер шрифта Veritas".into());
        }
        match &font["family"] {
            Value::String(family) if ["Proportional", "Monospace"].contains(&family.as_str()) => {}
            Value::Object(family)
                if family.len() == 1 && family.get("Name").is_some_and(Value::is_string) => {}
            _ => return Err("Неизвестное семейство шрифта Veritas".into()),
        }
    }
    let mut updater = read_json(update)?;
    if !updater.is_object() {
        return Err("Конфиг обновления Veritas должен быть объектом".into());
    }
    updater["version"] = json!(version);
    updater["beta"] = json!(false);
    updater["prompt_beta"] = json!(false);
    // Validate every input before writing any file. Original backups are kept once.
    for path in [&config_path, &state_path, update] {
        preserve(path)?;
    }
    atomic_json(&config_path, &config)?;
    atomic(&state_path, state.as_bytes())?;
    atomic_json(update, &updater)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_stock_values_never_change_config_files() {
        let root = std::env::temp_dir().join(format!(
            "nous-veritas-invalid-config-{}",
            std::process::id()
        ));
        let update = root.join("fake-game/veritas.update.json");
        let mut invalid = Vec::new();
        for theme in [
            Value::Array(vec![json!("Bogus"); 12]),
            Value::Array(vec![json!("Gray"); 11]),
            json!({"Custom":[0,1,2]}),
            Value::Array(vec![json!({"Custom":[0,1,256]}); 12]),
            Value::Array(vec![json!({"Custom":[0,1,2.0]}); 12]),
            Value::Array(vec![json!({"Custom":[0,1,2],"Gray":null}); 12]),
        ] {
            // a malformed palette already in the stock config
            invalid.push((
                PluginOptions::default(),
                json!({"future_option":123,"theme":theme}).to_string(),
            ));
        }
        invalid.push((
            PluginOptions {
                legend_text_style: "Bogus".into(),
                ..PluginOptions::default()
            },
            "{\"future_option\":123}".into(),
        ));
        let mut fonts = PluginOptions::default();
        fonts.font_sizes.insert("Bogus".into(), 14.0);
        invalid.push((fonts, "{\"future_option\":123}".into()));
        for (options, config) in invalid {
            std::fs::create_dir_all(root.join("config")).unwrap();
            std::fs::create_dir_all(root.join("data")).unwrap();
            std::fs::create_dir_all(update.parent().unwrap()).unwrap();
            let files = [
                (root.join("config/config.json"), config.into_bytes()),
                (root.join("data/state"), STATE.as_bytes().to_vec()),
                (update.clone(), b"{\"future_option\":456}".to_vec()),
            ];
            for (path, bytes) in &files {
                std::fs::write(path, bytes).unwrap();
            }
            assert!(
                write(&root, &update, "0.2.57", &options).is_err(),
                "accepted {options:?}"
            );
            for (path, bytes) in &files {
                assert_eq!(std::fs::read(path).unwrap(), *bytes);
                assert!(!path.with_extension("nous-original").exists());
                assert!(!path.with_extension("nous-tmp").exists());
            }
        }
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn keeps_valid_stock_palette() {
        let root =
            std::env::temp_dir().join(format!("nous-veritas-valid-config-{}", std::process::id()));
        let update = root.join("fake-game/veritas.update.json");
        for theme in [
            json!([
                "Gray", "EguiBlue", "Tomato", "Red", "Ruby", "Crimson", "Pink", "Plum", "Purple",
                "Violet", "Iris", "Indigo"
            ]),
            json!(["Blue","Cyan","Teal","Jade","Green","Grass","Brown","Bronze","Gold","Orange",{"Custom":[0,255,128]},"Gray"]),
        ] {
            std::fs::create_dir_all(root.join("config")).unwrap();
            std::fs::write(
                root.join("config/config.json"),
                json!({ "theme": theme }).to_string(),
            )
            .unwrap();
            write(&root, &update, "0.2.57", &PluginOptions::default()).unwrap();
            assert_eq!(
                read_json(&root.join("config/config.json")).unwrap()["theme"],
                theme
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn incomplete_stock_state_is_rejected_and_comments_remain_untouched() {
        assert!(patch_state("(hide_ui:false)", true).is_err());
        assert!(patch_state(&STATE.replace("show_menu:false,", ""), true).is_err());
        let text = STATE
            .replacen(
                "(",
                "( /* hide_ui:false /* show_menu:true */ */\n// show_settings:true\n",
                1,
            )
            .replace("hide_ui:true", "hide_ui:false");
        let patched = patch_state(&text, true).unwrap();
        assert!(patched.contains("/* hide_ui:false /* show_menu:true */ */"));
        assert!(patched.contains("// show_settings:true"));
        assert!(patched.contains(",hide_ui:true,"));
    }
    #[test]
    fn stock_state_edits_preserve_raw_strings_nested_fields_and_value_comments() {
        let text = STATE
            .replace("hide_ui:true", "hide_ui: /* retain this */ false")
            .replace(
                "custom_export_path:None",
                "custom_export_path:Some(r#\"C:\\hide_ui:false\"#)",
            )
            .replacen("(", "(future:(hide_ui:false,show_menu:true),", 1);
        let patched = patch_state(&text, true).unwrap();
        assert!(patched.contains("hide_ui: /* retain this */ true"));
        assert!(patched.contains("future:(hide_ui:false,show_menu:true)"));
        assert!(patched.contains("Some(r#\"C:\\hide_ui:false\"#)"));
        assert!(
            patch_state(
                &STATE.replace("show_character_damage:false", "show_character_damage:17"),
                true
            )
            .is_err()
        );
    }
    #[test]
    fn hiding_preserves_ron_enums_unknown_fields_and_quoted_text() {
        let s = STATE
            .replace("show_menu:false", "show_menu:true")
            .replace(
                "custom_export_path:None",
                r#"custom_export_path:Some("hide_ui: false")"#,
            )
            .replace("hide_ui:true", "hide_ui:false");
        let updated = patch_state(&s, true).unwrap();
        assert!(updated.contains("hide_ui:true"));
        assert!(updated.contains("graph_x_unit:Turn"));
        assert!(updated.contains(r#"Some("hide_ui: false")"#));
        assert!(updated.contains("show_menu:false"));
    }
    #[test]
    fn malformed_ron_is_rejected_before_overwrite() {
        assert!(patch_state("invalid (", true).is_err());
    }
    #[test]
    fn writes_stock_paths_and_keeps_originals() {
        let root = std::env::temp_dir().join(format!("nous-veritas-config-{}", std::process::id()));
        let update = root.join("fake-game/veritas.update.json");
        std::fs::create_dir_all(root.join("config")).unwrap();
        std::fs::write(
            root.join("config/config.json"),
            r#"{"future_option":123,"streamer_mode":true}"#,
        )
        .unwrap();
        write(&root, &update, "0.2.57", &PluginOptions::default()).unwrap();
        let config = read_json(&root.join("config/config.json")).unwrap();
        assert_eq!(config["future_option"], 123);
        assert_eq!(config["streamer_mode"], false);
        assert_eq!(read_json(&update).unwrap()["prompt_beta"], false);
        assert!(root.join("config/config.nous-original").exists());
        assert!(read(&root).unwrap().suppress_native);
        std::fs::remove_dir_all(root).unwrap();
    }
}

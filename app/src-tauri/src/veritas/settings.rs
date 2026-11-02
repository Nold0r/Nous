use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OverlaySettings {
    pub enabled: bool,
    pub click_through: bool,
    pub scale: f64,
    pub opacity: f64,
    pub font_sizes: BTreeMap<String, f64>,
    pub legend_size: String,
    pub pie_opacity: f64,
    pub views: BTreeMap<String, bool>,
    pub columns: Vec<String>,
    pub metric: String,
    pub breakdown_scope: String,
    pub breakdown_chart: String,
    pub character_id: Option<u32>,
    pub show_legend: bool,
    pub show_breakdown_table: bool,
    pub graph_unit: String,
    pub streamer_mode: bool,
    pub streamer_message: String,
    pub auto_export: bool,
    pub date_folders: bool,
    pub export_directory: Option<String>,
    /// Widget id -> top-left corner as a fraction of the game's client area.
    pub widget_positions: BTreeMap<String, [f64; 2]>,
}
impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            click_through: true,
            scale: 1.0,
            opacity: 0.86,
            font_sizes: [
                ("body", 14.0),
                ("heading", 20.0),
                ("small", 12.0),
                ("legend", 14.0),
            ]
            .into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect(),
            legend_size: "body".into(),
            pie_opacity: 0.8,
            views: [
                ("legend", true),
                ("characters", true),
                ("breakdown", false),
                ("enemies", false),
                ("metrics", true),
                ("graph", false),
                ("logs", false),
            ]
            .into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect(),
            columns: ["damage", "dpav", "share"].map(String::from).to_vec(),
            metric: "damage".into(),
            breakdown_scope: "team".into(),
            breakdown_chart: "pie".into(),
            character_id: None,
            show_legend: true,
            show_breakdown_table: true,
            graph_unit: "turn".into(),
            streamer_mode: false,
            streamer_message: "Nous".into(),
            auto_export: false,
            date_folders: true,
            export_directory: None,
            widget_positions: BTreeMap::new(),
        }
    }
}
impl OverlaySettings {
    pub fn validate(&self) -> Result<(), String> {
        fn range(v: f64, low: f64, high: f64) -> bool {
            v.is_finite() && (low..=high).contains(&v)
        }
        if !range(self.scale, 0.5, 2.5)
            || !range(self.opacity, 0.1, 1.0)
            || !range(self.pie_opacity, 0.0, 1.0)
            || self.font_sizes.values().any(|v| !range(*v, 8.0, 48.0))
        {
            return Err("Размер, масштаб или прозрачность вне допустимого диапазона".into());
        }
        if self.widget_positions.len() > 16
            || self
                .widget_positions
                .iter()
                .any(|(id, [x, y])| id.len() > 32 || !range(*x, 0.0, 1.0) || !range(*y, 0.0, 1.0))
        {
            return Err("Неверное положение виджета".into());
        }
        if self.streamer_message.len() > 500 {
            return Err("Слишком длинное значение настройки".into());
        }
        Ok(())
    }
}

/// Widgets in display order, with where they start before the player moves them.
pub const WIDGETS: [(&str, [f64; 2]); 7] = [
    ("legend", [0.72, 0.05]),
    ("characters", [0.72, 0.40]),
    ("breakdown", [0.02, 0.40]),
    ("enemies", [0.35, 0.03]),
    ("metrics", [0.02, 0.05]),
    ("graph", [0.02, 0.62]),
    ("logs", [0.72, 0.75]),
];
impl OverlaySettings {
    pub fn widget_position(&self, id: &str) -> [f64; 2] {
        self.widget_positions.get(id).copied().unwrap_or_else(|| {
            WIDGETS
                .iter()
                .find(|(w, _)| *w == id)
                .map_or([0.05, 0.05], |(_, p)| *p)
        })
    }
}

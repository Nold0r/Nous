use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OverlaySettings {
    pub enabled: bool,
    pub click_through: bool,
    pub scale: f64,
    pub opacity: f64,
    pub width: f64,
    pub height: f64,
    pub anchor: String,
    pub offset_x: f64,
    pub offset_y: f64,
    pub theme: String,
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
}
impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            click_through: true,
            scale: 1.0,
            opacity: 0.86,
            width: 520.0,
            height: 620.0,
            anchor: "top-right".into(),
            offset_x: 24.0,
            offset_y: 24.0,
            theme: "nous".into(),
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
            || !range(self.width, 300.0, 1400.0)
            || !range(self.height, 180.0, 1400.0)
            || !range(self.offset_x, 0.0, 4000.0)
            || !range(self.offset_y, 0.0, 4000.0)
            || self.font_sizes.values().any(|v| !range(*v, 8.0, 48.0))
        {
            return Err("Размер, масштаб или прозрачность вне допустимого диапазона".into());
        }
        if !["top-right", "top-left", "bottom-right", "bottom-left"].contains(&self.anchor.as_str())
        {
            return Err("Неизвестная привязка окна".into());
        }
        if self.streamer_message.len() > 500 {
            return Err("Слишком длинное значение настройки".into());
        }
        Ok(())
    }
}

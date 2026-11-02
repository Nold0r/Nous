//! Shell preferences, separate from SDK accounts. Native writes complete before
//! the UI accepts a change, so an immediate restart cannot lose dev selection.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Configuration {
    disabled: Vec<String>,
    settings: BTreeMap<String, serde_json::Map<String, serde_json::Value>>,
}
fn load(dir: &Path) -> Result<Option<Configuration>, String> {
    match std::fs::read(dir.join("modules.json")) {
        Ok(data) => serde_json::from_slice(&data)
            .map(Some)
            .map_err(|e| format!("modules.json: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("modules.json: {e}")),
    }
}
fn save(dir: &Path, configuration: &Configuration) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let data = serde_json::to_vec(configuration).map_err(|e| e.to_string())?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = dir.join(format!("modules-{stamp}.tmp"));
    std::fs::write(&temporary, data)
        .and_then(|_| std::fs::rename(&temporary, dir.join("modules.json")))
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn get_module_configuration(app: AppHandle) -> Result<Option<Configuration>, String> {
    load(&app.path().app_data_dir().map_err(|e| e.to_string())?)
}
#[tauri::command]
pub fn set_module_configuration(
    app: AppHandle,
    configuration: Configuration,
) -> Result<(), String> {
    save(
        &app.path().app_data_dir().map_err(|e| e.to_string())?,
        &configuration,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_and_module_settings_survive_a_new_store_read() {
        let dir = std::env::temp_dir().join(format!("nous-module-config-{}", std::process::id()));
        assert_eq!(load(&dir).unwrap(), None);
        let first = Configuration {
            disabled: vec!["map".into(), "overview".into()],
            settings: BTreeMap::from([(
                "map".into(),
                serde_json::from_value(serde_json::json!({"zoom": 2})).unwrap(),
            )]),
        };
        save(&dir, &first).unwrap();
        assert_eq!(load(&dir).unwrap(), Some(first));
        let second = Configuration {
            disabled: vec!["relics".into()],
            ..Default::default()
        };
        save(&dir, &second).unwrap();
        assert_eq!(load(&dir).unwrap(), Some(second));
        assert_eq!(
            std::fs::read_dir(&dir).unwrap().count(),
            1,
            "no partial temp files remain"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}

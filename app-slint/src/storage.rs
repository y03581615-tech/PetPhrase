use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Phrase {
    pub id: String,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Group {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub phrases: Vec<Phrase>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PhraseData {
    pub groups: Vec<Group>,
}

impl Default for PhraseData {
    fn default() -> Self {
        PhraseData {
            groups: vec![Group {
                id: "g-default".into(),
                name: "常用".into(),
                icon: Some("star".into()),
                phrases: vec![
                    Phrase { id: "p-1".into(), text: "收到,马上处理".into() },
                    Phrase { id: "p-2".into(), text: "好的,没问题".into() },
                    Phrase {
                        id: "p-3".into(),
                        text: "您好,感谢您的反馈,我们已经记录了这个问题,会尽快给您答复。".into(),
                    },
                ],
            }],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Settings {
    #[serde(default = "default_pet_id")]
    pub pet_id: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub pet_pos: Option<(i32, i32)>,
    #[serde(default)]
    pub last_group: Option<String>,
    #[serde(default)]
    pub custom_pet_dir: Option<String>,
}

fn default_pet_id() -> String {
    "default".into()
}
fn default_theme() -> String {
    "acrylic".into()
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            pet_id: default_pet_id(),
            theme: default_theme(),
            pet_pos: None,
            last_group: None,
            custom_pet_dir: None,
        }
    }
}

const PHRASES_FILE: &str = "phrases.json";
const BACKUP_FILE: &str = "phrases.backup.json";
const SETTINGS_FILE: &str = "settings.json";

fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// 写临时文件后 rename,避免半写状态。
pub fn atomic_write(path: &Path, contents: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, contents)?;
    // Windows 上 rename 到已存在目标会失败,先删旧
    let _ = fs::remove_file(path);
    fs::rename(&tmp, path)
}

/// 主文件 → 备份 → 默认;从备份恢复时回写主文件。
pub fn load_phrases(dir: &Path) -> PhraseData {
    let main = dir.join(PHRASES_FILE);
    if let Some(data) = read_json::<PhraseData>(&main) {
        return data;
    }
    if let Some(data) = read_json::<PhraseData>(&dir.join(BACKUP_FILE)) {
        let _ = save_phrases(dir, &data); // 恢复主文件
        return data;
    }
    PhraseData::default()
}

pub fn save_phrases(dir: &Path, data: &PhraseData) -> io::Result<()> {
    let json = serde_json::to_string_pretty(data).map_err(io::Error::other)?;
    atomic_write(&dir.join(PHRASES_FILE), &json)
}

/// 启动时调用:当前主文件可读则拷为备份。
pub fn backup_phrases(dir: &Path) {
    let main = dir.join(PHRASES_FILE);
    if read_json::<PhraseData>(&main).is_some() {
        let _ = fs::copy(&main, dir.join(BACKUP_FILE));
    }
}

pub fn load_settings(dir: &Path) -> Settings {
    read_json::<Settings>(&dir.join(SETTINGS_FILE)).unwrap_or_default()
}

pub fn save_settings(dir: &Path, s: &Settings) -> io::Result<()> {
    let json = serde_json::to_string_pretty(s).map_err(io::Error::other)?;
    atomic_write(&dir.join(SETTINGS_FILE), &json)
}

pub fn export_phrases(dir: &Path, dest: &Path) -> io::Result<()> {
    let data = load_phrases(dir);
    let json = serde_json::to_string_pretty(&data).map_err(io::Error::other)?;
    atomic_write(dest, &json)
}

/// 校验外部文件后覆盖存储,返回导入结果。
pub fn import_phrases(dir: &Path, src: &Path) -> io::Result<PhraseData> {
    let text = fs::read_to_string(src)?;
    let data: PhraseData = serde_json::from_str(&text)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("文件格式不正确: {e}")))?;
    save_phrases(dir, &data)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn save_then_load_roundtrip() {
        let dir = tempdir().unwrap();
        let mut data = PhraseData::default();
        data.groups[0].phrases.push(Phrase { id: "x".into(), text: "测试".into() });
        save_phrases(dir.path(), &data).unwrap();
        assert_eq!(load_phrases(dir.path()), data);
    }

    #[test]
    fn load_missing_returns_default_with_sample_group() {
        let dir = tempdir().unwrap();
        let data = load_phrases(dir.path());
        assert_eq!(data.groups.len(), 1);
        assert_eq!(data.groups[0].name, "常用");
        assert!(!data.groups[0].phrases.is_empty());
    }

    #[test]
    fn corrupt_json_recovers_from_backup_and_rewrites_main() {
        let dir = tempdir().unwrap();
        let data = PhraseData::default();
        save_phrases(dir.path(), &data).unwrap();
        backup_phrases(dir.path());
        fs::write(dir.path().join("phrases.json"), "{broken!!").unwrap();
        assert_eq!(load_phrases(dir.path()), data);
        // 主文件已被修复
        let repaired = fs::read_to_string(dir.path().join("phrases.json")).unwrap();
        assert!(serde_json::from_str::<PhraseData>(&repaired).is_ok());
    }

    #[test]
    fn atomic_write_leaves_no_tmp() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("phrases.json");
        atomic_write(&target, "{}").unwrap();
        atomic_write(&target, "{\"groups\":[]}").unwrap(); // 覆盖已有文件
        assert!(!target.with_extension("tmp").exists());
        assert_eq!(fs::read_to_string(&target).unwrap(), "{\"groups\":[]}");
    }

    #[test]
    fn settings_roundtrip_and_default() {
        let dir = tempdir().unwrap();
        let s = load_settings(dir.path());
        assert_eq!(s.pet_id, "default");
        assert_eq!(s.theme, "acrylic");
        let changed = Settings { pet_pos: Some((100, 200)), ..s };
        save_settings(dir.path(), &changed).unwrap();
        assert_eq!(load_settings(dir.path()), changed);
    }

    #[test]
    fn import_rejects_invalid_json() {
        let dir = tempdir().unwrap();
        let bad = dir.path().join("bad.json");
        fs::write(&bad, "not json").unwrap();
        assert!(import_phrases(dir.path(), &bad).is_err());
    }

    #[test]
    fn export_then_import_roundtrip() {
        let dir = tempdir().unwrap();
        let data = PhraseData::default();
        save_phrases(dir.path(), &data).unwrap();
        let out = dir.path().join("out.json");
        export_phrases(dir.path(), &out).unwrap();
        let dir2 = tempdir().unwrap();
        assert_eq!(import_phrases(dir2.path(), &out).unwrap(), data);
    }
}

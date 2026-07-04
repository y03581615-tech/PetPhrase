use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct PetInfo {
    pub id: String,
    pub name: String,
    /// spritesheet 绝对路径;error 非空时为空串
    pub spritesheet: String,
    pub error: Option<String>,
}

/// 单个宠物目录 → PetInfo。宽松校验:pet.json 可缺 name(用目录名),
/// spritesheet.webp/png 必须存在。尺寸/网格由前端加载图片时推算。
fn load_pet(dir: &Path) -> Option<PetInfo> {
    if !dir.is_dir() {
        return None;
    }
    let id = dir.file_name()?.to_string_lossy().to_string();
    let mut name = id.clone();
    let mut error: Option<String> = None;

    match fs::read_to_string(dir.join("pet.json")) {
        Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(meta) => {
                // 兼容两种字段:官方素材用 displayName,规范文档用 name
                if let Some(n) = meta
                    .get("displayName")
                    .or_else(|| meta.get("name"))
                    .and_then(|v| v.as_str())
                {
                    name = n.to_string();
                }
            }
            Err(e) => error = Some(format!("pet.json 解析失败: {e}")),
        },
        Err(_) => error = Some("缺少 pet.json".into()),
    }

    let spritesheet = ["spritesheet.webp", "spritesheet.png"]
        .iter()
        .map(|f| dir.join(f))
        .find(|p| p.is_file());

    let spritesheet = match spritesheet {
        Some(p) => p.to_string_lossy().to_string(),
        None => {
            error.get_or_insert("缺少 spritesheet.webp/png".into());
            String::new()
        }
    };

    Some(PetInfo { id, name, spritesheet, error })
}

/// 依序扫描多个根目录,每个根目录下的一级子目录 = 一个宠物包。
/// 同 id 先到先得(内置目录优先级最高)。
pub fn scan_pets(roots: &[&Path]) -> Vec<PetInfo> {
    let mut pets: Vec<PetInfo> = Vec::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else { continue };
        for entry in entries.flatten() {
            if let Some(pet) = load_pet(&entry.path()) {
                if !pets.iter().any(|p| p.id == pet.id) {
                    pets.push(pet);
                }
            }
        }
    }
    pets
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_pet(root: &Path, id: &str, json: Option<&str>, sheet: Option<&str>) {
        let dir = root.join(id);
        fs::create_dir_all(&dir).unwrap();
        if let Some(j) = json {
            fs::write(dir.join("pet.json"), j).unwrap();
        }
        if let Some(f) = sheet {
            fs::write(dir.join(f), b"fake-image").unwrap();
        }
    }

    #[test]
    fn display_name_takes_precedence() {
        let root = tempdir().unwrap();
        make_pet(
            root.path(),
            "kun-like",
            Some(r#"{"id":"Kun-like","displayName":"Kun Like","name":"ignored"}"#),
            Some("spritesheet.webp"),
        );
        let pets = scan_pets(&[root.path()]);
        assert_eq!(pets[0].name, "Kun Like");
    }

    #[test]
    fn scans_valid_pet() {
        let root = tempdir().unwrap();
        make_pet(root.path(), "kun-like", Some(r#"{"name":"Kun"}"#), Some("spritesheet.webp"));
        let pets = scan_pets(&[root.path()]);
        assert_eq!(pets.len(), 1);
        assert_eq!(pets[0].name, "Kun");
        assert_eq!(pets[0].id, "kun-like");
        assert!(pets[0].error.is_none());
        assert!(pets[0].spritesheet.ends_with("spritesheet.webp"));
    }

    #[test]
    fn missing_spritesheet_yields_error() {
        let root = tempdir().unwrap();
        make_pet(root.path(), "broken", Some(r#"{"name":"X"}"#), None);
        let pets = scan_pets(&[root.path()]);
        assert_eq!(pets[0].error.as_deref(), Some("缺少 spritesheet.webp/png"));
    }

    #[test]
    fn bad_json_yields_error_with_dirname_as_name() {
        let root = tempdir().unwrap();
        make_pet(root.path(), "oops", Some("{bad"), Some("spritesheet.png"));
        let pets = scan_pets(&[root.path()]);
        assert_eq!(pets[0].name, "oops");
        assert!(pets[0].error.as_deref().unwrap().contains("解析失败"));
    }

    #[test]
    fn merges_roots_in_order_first_wins() {
        let a = tempdir().unwrap();
        let b = tempdir().unwrap();
        make_pet(a.path(), "dup", Some(r#"{"name":"FromA"}"#), Some("spritesheet.png"));
        make_pet(b.path(), "dup", Some(r#"{"name":"FromB"}"#), Some("spritesheet.png"));
        make_pet(b.path(), "only-b", Some(r#"{"name":"B"}"#), Some("spritesheet.png"));
        let pets = scan_pets(&[a.path(), b.path()]);
        assert_eq!(pets.len(), 2);
        assert_eq!(pets.iter().find(|p| p.id == "dup").unwrap().name, "FromA");
    }

    #[test]
    fn nonexistent_root_is_skipped() {
        let pets = scan_pets(&[Path::new("Z:/no/such/dir")]);
        assert!(pets.is_empty());
    }
}

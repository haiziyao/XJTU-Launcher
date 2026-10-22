//! Roles, skills and preset tasks live in ~/.xjtu-launcher/agent/presets.json.
use serde::{Deserialize, Serialize};
use std::path::Path;

const MAX_ITEMS: usize = 64;
const MAX_INSTRUCTIONS: usize = 8_000;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Role {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub instructions: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Skill {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub instructions: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Task {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub prompt: String,
    #[serde(default)]
    pub role_id: Option<String>,
    #[serde(default)]
    pub skill_ids: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Presets {
    #[serde(default = "version")]
    pub version: u32,
    #[serde(default)]
    pub roles: Vec<Role>,
    #[serde(default)]
    pub skills: Vec<Skill>,
    #[serde(default)]
    pub tasks: Vec<Task>,
}

fn version() -> u32 {
    1
}

impl Default for Presets {
    fn default() -> Self {
        Self {
            version: 1,
            roles: Vec::new(),
            skills: Vec::new(),
            tasks: Vec::new(),
        }
    }
}

impl Presets {
    fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || self.roles.len() > MAX_ITEMS
            || self.skills.len() > MAX_ITEMS
            || self.tasks.len() > MAX_ITEMS
        {
            return Err("预设数量超限或版本不受支持".into());
        }
        for role in &self.roles {
            valid(&role.id, &role.name)?;
            if role.instructions.chars().count() > MAX_INSTRUCTIONS {
                return Err("角色指令过长".into());
            }
        }
        for skill in &self.skills {
            valid(&skill.id, &skill.name)?;
            if skill.instructions.chars().count() > MAX_INSTRUCTIONS {
                return Err("Skill 指令过长".into());
            }
        }
        for task in &self.tasks {
            valid(&task.id, &task.name)?;
            if task.prompt.trim().is_empty() || task.prompt.chars().count() > 4_000 {
                return Err("预设任务内容需要 1–4000 个字符".into());
            }
        }
        let mut ids = std::collections::HashSet::new();
        for id in self
            .roles
            .iter()
            .map(|item| &item.id)
            .chain(self.skills.iter().map(|item| &item.id))
            .chain(self.tasks.iter().map(|item| &item.id))
        {
            if !ids.insert(id) {
                return Err("预设 ID 重复".into());
            }
        }
        Ok(())
    }
}

fn valid(id: &str, name: &str) -> Result<(), String> {
    if uuid::Uuid::parse_str(id).is_err() || name.trim().is_empty() || name.chars().count() > 40 {
        return Err("预设名称或标识无效".into());
    }
    Ok(())
}

pub fn load(path: &Path) -> Result<Presets, String> {
    if !path.exists() {
        return Ok(Presets::default());
    }
    let bytes = std::fs::read(path).map_err(|_| "无法读取 Agent 预设".to_string())?;
    let presets: Presets = serde_json::from_slice(&bytes)
        .map_err(|_| "Agent 预设损坏，已保留原文件，请修复后重试".to_string())?;
    presets.validate()?;
    Ok(presets)
}

pub fn save(path: &Path, presets: &Presets) -> Result<(), String> {
    presets.validate()?;
    crate::config::write_atomic(
        path,
        &serde_json::to_vec_pretty(presets).map_err(|_| "无法序列化 Agent 预设")?,
    )
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Input {
    Role {
        #[serde(default)]
        id: Option<String>,
        name: String,
        #[serde(default)]
        description: String,
        instructions: String,
    },
    Skill {
        #[serde(default)]
        id: Option<String>,
        name: String,
        #[serde(default)]
        description: String,
        instructions: String,
    },
    Task {
        #[serde(default)]
        id: Option<String>,
        name: String,
        #[serde(default)]
        description: String,
        prompt: String,
        #[serde(default)]
        role_id: Option<String>,
        #[serde(default)]
        skill_ids: Vec<String>,
    },
}

pub fn upsert(presets: &mut Presets, input: Input) -> Result<String, String> {
    let id =
        |existing: Option<String>| existing.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    match input {
        Input::Role {
            id: existing,
            name,
            description,
            instructions,
        } => {
            let role = Role {
                id: id(existing),
                name,
                description,
                instructions,
            };
            let id = role.id.clone();
            match presets.roles.iter_mut().find(|item| item.id == id) {
                Some(item) => *item = role,
                None => presets.roles.push(role),
            }
            Ok(id)
        }
        Input::Skill {
            id: existing,
            name,
            description,
            instructions,
        } => {
            let skill = Skill {
                id: id(existing),
                name,
                description,
                instructions,
            };
            let id = skill.id.clone();
            match presets.skills.iter_mut().find(|item| item.id == id) {
                Some(item) => *item = skill,
                None => presets.skills.push(skill),
            }
            Ok(id)
        }
        Input::Task {
            id: existing,
            name,
            description,
            prompt,
            role_id,
            skill_ids,
        } => {
            let task = Task {
                id: id(existing),
                name,
                description,
                prompt,
                role_id,
                skill_ids,
            };
            let id = task.id.clone();
            match presets.tasks.iter_mut().find(|item| item.id == id) {
                Some(item) => *item = task,
                None => presets.tasks.push(task),
            }
            Ok(id)
        }
    }
}

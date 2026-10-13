//! 串联器：读定义（配置），把定义展开成一串基础单元的活动，一步步做完。
//!
//! 定义说的是我们的话（读投递邮件 / 看谁进群了 / 发问卷调研），基础单元说的是飞书的话
//! （`mail +triage` / `base +record-list`）。这一层只干两件事：**翻译**与**串联**——
//! 把参数模板换成真值、把上一步的产出喂给下一步、按定义声明把产出起成我们的名字。
//!
//! ```text
//! 一个定义 = 有序的步骤 + 一张「出」表（可选）+ 一条增量声明（可选）
//!   步骤   一步 = 一个基础单元的动作 + 参数，参数里可以写 {{选择器}}；
//!          as: 给这一步的产出起名，后面的步骤就能取用（{{摘要.messages[].message_id}}）
//!   出     行: 说哪一段是行，字段: 说每行按什么起名——左边是我们的话，右边是飞书的字段
//!   增量   比: 日期——只留比游标新的行，新游标取这批里最新的值；游标由调用方保管
//! ```
//!
//! 上下文里的东西：调用方给的变量（邮箱、文件夹、候选人…）、各步骤的产出（按 `as` 起的名），
//! 以及保留名 `游标`（增量用）。

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::backend::Backend;

/// 增量的上下文变量名：调用方给、调用方存。
pub const CURSOR: &str = "游标";

#[derive(Debug, Deserialize)]
struct RawDefinition {
    #[serde(rename = "名称")]
    name: String,
    #[serde(rename = "说明", default)]
    note: String,
    #[serde(rename = "步骤", default)]
    steps: Vec<RawStep>,
    #[serde(rename = "出")]
    out: Option<RawOut>,
    #[serde(rename = "增量")]
    increment: Option<RawIncrement>,
}

#[derive(Debug, Deserialize)]
struct RawStep {
    #[serde(rename = "动作")]
    action: String,
    #[serde(rename = "参数", default)]
    params: serde_yaml::Mapping,
    #[serde(rename = "as", default)]
    as_name: String,
}

#[derive(Debug, Deserialize)]
struct RawOut {
    #[serde(rename = "行", default)]
    rows_from: String,
    #[serde(rename = "字段", default)]
    fields: serde_yaml::Mapping,
}

#[derive(Debug, Deserialize)]
struct RawIncrement {
    #[serde(rename = "比", default)]
    by: String,
}

/// 一步：基础单元的一个动作 + 参数；起个名，后面的步骤能取用它的产出。
#[derive(Debug, Clone)]
pub struct Step {
    pub action: String,
    pub params: Map<String, Value>,
    pub as_name: String,
}

/// 一条定义：我们的话 → 一串飞书活动。
#[derive(Debug, Clone)]
pub struct Definition {
    pub name: String,
    pub note: String,
    pub steps: Vec<Step>,
    pub rows_from: String,
    pub fields: Vec<(String, String)>,
    pub increment_by: String,
}

/// 一次定义的产出。
#[derive(Debug, Clone, Default)]
pub struct Outcome {
    pub rows: Vec<Map<String, Value>>,
    pub single: Map<String, Value>,
    pub cursor: Option<String>,
    pub steps: BTreeMap<String, Value>,
}

impl Outcome {
    /// 所有行里所有字段的值——「包含某邮箱」「非空」这类判断看它。
    pub fn values(&self) -> Vec<String> {
        let mut out = Vec::new();
        for row in &self.rows {
            for value in row.values() {
                if !crate::resources::action::is_absent(value) {
                    out.push(crate::resources::action::text(value));
                }
            }
        }
        out
    }
}

/// 定义表：键是定义名（就是我们的话）。
pub type Definitions = BTreeMap<String, Definition>;

#[derive(Debug, Deserialize)]
struct RawFile {
    #[serde(rename = "定义")]
    definitions: Vec<RawDefinition>,
}

/// 读定义（YAML 文本）。
pub fn parse(text: &str) -> Result<Definitions, String> {
    let file: RawFile = serde_yaml::from_str(text).map_err(|err| format!("定义读不通：{err}"))?;
    let mut defs = Definitions::new();
    for raw in file.definitions {
        let steps = raw
            .steps
            .into_iter()
            .map(|step| {
                let params = serde_json::to_value(&step.params)
                    .ok()
                    .and_then(|value| value.as_object().cloned())
                    .unwrap_or_default();
                Step {
                    action: step.action,
                    params,
                    as_name: step.as_name,
                }
            })
            .collect();
        let fields = raw
            .out
            .as_ref()
            .and_then(|out| serde_json::to_value(&out.fields).ok())
            .and_then(|value| value.as_object().cloned())
            .map(|map| {
                map.into_iter()
                    .map(|(key, value)| (key, crate::resources::action::text(&value)))
                    .collect()
            })
            .unwrap_or_default();
        defs.insert(
            raw.name.clone(),
            Definition {
                name: raw.name,
                note: raw.note,
                steps,
                rows_from: raw
                    .out
                    .as_ref()
                    .map(|out| out.rows_from.clone())
                    .unwrap_or_default(),
                fields,
                increment_by: raw
                    .increment
                    .as_ref()
                    .map(|inc| inc.by.clone())
                    .unwrap_or_default(),
            },
        );
    }
    Ok(defs)
}

/// 读定义文件（YAML）。
pub fn load(path: &Path) -> Result<Definitions, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("读不到定义文件 {}：{err}", path.display()))?;
    parse(&text)
}

/// 从上下文里取一个值：`a.b` 逐层取；`a[].b` 把 a 的每个元素都取一遍；
/// `a.英文\|a.中文` 取第一个取得到的（表列名两种写法都认）。
pub fn pick(context: &Value, path: &str) -> Option<Value> {
    for alternative in path.split('|') {
        let value = walk(context, alternative.trim());
        if !value.is_null() {
            if let Value::String(text) = &value
                && text.is_empty()
            {
                continue;
            }
            if let Value::Array(items) = &value
                && items.is_empty()
            {
                continue;
            }
            return Some(value);
        }
    }
    None
}

fn walk(value: &Value, path: &str) -> Value {
    if path.is_empty() {
        return value.clone();
    }
    let (head, rest) = match path.split_once('.') {
        Some((head, rest)) => (head, rest),
        None => (path, ""),
    };
    let pluck = head.ends_with("[]");
    let key = if pluck { &head[..head.len() - 2] } else { head };
    let current = match value {
        Value::Object(map) => map.get(key).cloned().unwrap_or(Value::Null),
        other => other.get(key).cloned().unwrap_or(Value::Null),
    };
    if pluck {
        match current {
            Value::Array(items) => {
                Value::Array(items.iter().map(|item| walk(item, rest)).collect())
            }
            _ => Value::Array(Vec::new()),
        }
    } else {
        walk(&current, rest)
    }
}

/// 把参数里的 `{{…}}` 换成真值。整串就是一个占位符的，保持原类型（列表还是列表）。
pub fn resolve(value: &Value, context: &Map<String, Value>) -> Value {
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            if let Some(inner) = trimmed
                .strip_prefix("{{")
                .and_then(|s| s.strip_suffix("}}"))
                && !inner.contains("{{")
            {
                return pick(&Value::Object(context.clone()), inner).unwrap_or(Value::Null);
            }
            replace_placeholders(text, context)
        }
        Value::Array(items) => {
            Value::Array(items.iter().map(|item| resolve(item, context)).collect())
        }
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, item)| (key.clone(), resolve(item, context)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn replace_placeholders(text: &str, context: &Map<String, Value>) -> Value {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find("}}") else {
            out.push_str(&rest[start..]);
            return Value::String(out);
        };
        let path = &rest[start + 2..start + end];
        let value = pick(&Value::Object(context.clone()), path).unwrap_or(Value::Null);
        out.push_str(&crate::resources::action::text(&value));
        rest = &rest[start + end + 2..];
    }
    out.push_str(rest);
    Value::String(out)
}

pub struct Engine<'a> {
    backend: &'a Backend,
    defs: Definitions,
}

impl<'a> Engine<'a> {
    pub fn new(backend: &'a Backend, defs: Definitions) -> Self {
        Engine { backend, defs }
    }

    /// 跑一个定义：一步接一步，最后按「出」表起名、按增量声明筛一遍。
    pub fn call(&self, name: &str, context: &Map<String, Value>) -> Result<Outcome, String> {
        let definition = self
            .defs
            .get(name)
            .ok_or_else(|| format!("没有这条定义：{name}"))?;
        let mut ctx = context.clone();
        let mut steps: BTreeMap<String, Value> = BTreeMap::new();
        let mut last = Value::Null;
        for step in &definition.steps {
            let params: Map<String, Value> = step
                .params
                .iter()
                .map(|(key, value)| (key.clone(), resolve(value, &ctx)))
                .collect();
            let payload = self.backend.call(&step.action, params)?;
            let value = payload.value().clone();
            steps.insert(
                if step.as_name.is_empty() {
                    step.action.clone()
                } else {
                    step.as_name.clone()
                },
                value.clone(),
            );
            if !step.as_name.is_empty() {
                ctx.insert(step.as_name.clone(), value.clone());
            }
            last = value;
        }

        let mut outcome = Outcome {
            steps,
            ..Default::default()
        };
        if !definition.rows_from.is_empty() {
            let rows = pick(&Value::Object(ctx.clone()), &definition.rows_from)
                .and_then(|value| value.as_array().cloned())
                .unwrap_or_default();
            outcome.rows = rows
                .iter()
                .map(|row| {
                    definition
                        .fields
                        .iter()
                        .map(|(our, selector)| {
                            (our.clone(), pick(row, selector).unwrap_or(Value::Null))
                        })
                        .collect()
                })
                .collect();
        } else if definition.fields.is_empty() {
            if let Value::Object(map) = last {
                outcome.single = map;
            }
        } else {
            outcome.single = definition
                .fields
                .iter()
                .map(|(our, selector)| (our.clone(), pick(&last, selector).unwrap_or(Value::Null)))
                .collect();
        }

        if !definition.increment_by.is_empty() {
            let key = &definition.increment_by;
            let cursor_in = context
                .get(CURSOR)
                .map(crate::resources::action::text)
                .unwrap_or_default();
            if !cursor_in.is_empty() {
                outcome.rows.retain(|row| {
                    crate::resources::action::text(row.get(key).unwrap_or(&Value::Null)) > cursor_in
                });
            }
            let newest = outcome
                .rows
                .iter()
                .map(|row| crate::resources::action::text(row.get(key).unwrap_or(&Value::Null)))
                .max()
                .unwrap_or_default();
            let cursor = if newest.is_empty() { cursor_in } else { newest };
            if !cursor.is_empty() {
                outcome.cursor = Some(cursor);
            }
        }
        Ok(outcome)
    }
}

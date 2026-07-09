use std::borrow::Cow;

use crate::{
    ToolSpec,
    tool::{Tool, safe_path},
};
use anyhow::{Context as _, Result};
use async_openai::types::chat::{ChatCompletionTool, ChatCompletionTools, FunctionObjectArgs};
use async_trait::async_trait;
use serde_json::Value;
use tokio::fs;

pub struct ReadFileTool;

pub fn read_file_tool() -> Box<dyn Tool> {
    Box::new(ReadFileTool)
}

#[async_trait]
impl Tool for ReadFileTool {
    async fn invoke(&self, input: &Value) -> Result<String> {
        let path = input
            .get("path")
            .and_then(|value| value.as_str())
            .context("Invalid path")?;

        let path = safe_path(path)?;

        let limit = input.get("limit").and_then(|value| value.as_u64());

        let content = fs::read_to_string(path)
            .await
            .map_err(|error| anyhow::anyhow!("Error: {}", error))?;

        let mut lines: Vec<String> = content.lines().map(|line| line.to_string()).collect();

        if let Some(limit) = limit
            && (limit as usize) < lines.len()
        {
            let remaining = lines.len() - limit as usize;
            lines.truncate(limit as usize);
            lines.push(format!("...({} more lines)", remaining));
        }

        let result = lines.join("\n");

        Ok(result.chars().take(50000).collect())
    }

    fn name(&self) -> Cow<'_, str> {
        "read_file".into()
    }

    fn tool_spec(&self) -> ToolSpec {
        ChatCompletionTools::Function(ChatCompletionTool {
            function: FunctionObjectArgs::default()
                .name("read_file")
                .description("Read file contents.")
                .parameters(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string"
                        },
                        "limit": {
                            "type": "integer"
                        }
                    },
                    "required": ["path"]
                }))
                .build()
                .expect("valid read_file tool schema"),
        })
    }
}

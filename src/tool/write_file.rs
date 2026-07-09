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

pub struct WriteFileTool;

pub fn write_file_tool() -> Box<dyn Tool> {
    Box::new(WriteFileTool)
}

#[async_trait]
impl Tool for WriteFileTool {
    async fn invoke(&self, input: &Value) -> Result<String> {
        let path = input
            .get("path")
            .and_then(|value| value.as_str())
            .context("Invalid path")?;

        let path = safe_path(path)?;

        let content = input
            .get("content")
            .and_then(|value| value.as_str())
            .context("Invalid content")?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.ok();
        }

        fs::write(&path, content)
            .await
            .map_err(|error| anyhow::anyhow!("Error: {}", error))?;

        Ok(format!(
            "Wrote {} bytes to {}",
            content.len(),
            path.display()
        ))
    }

    fn name(&self) -> Cow<'_, str> {
        "write_file".into()
    }

    fn tool_spec(&self) -> ToolSpec {
        ChatCompletionTools::Function(ChatCompletionTool {
            function: FunctionObjectArgs::default()
                .name("write_file")
                .description("Write content to file.")
                .parameters(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string"
                        },
                        "content": {
                            "type": "string"
                        }
                    },
                    "required": ["path", "content"]
                }))
                .build()
                .expect("valid write_file tool schema"),
        })
    }
}

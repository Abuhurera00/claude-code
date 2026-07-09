use std::{borrow::Cow, time::Duration};

use crate::{ToolSpec, tool::Tool};
use anyhow::{Context as _, Result};
use async_openai::types::chat::{ChatCompletionTool, ChatCompletionTools, FunctionObjectArgs};
use async_trait::async_trait;
use serde_json::Value;
use tokio::{process::Command, time::timeout};

pub struct BashTool;

pub fn bash_tool() -> Box<dyn Tool> {
    Box::new(BashTool)
}

#[async_trait]
impl Tool for BashTool {
    async fn invoke(&self, input: &Value) -> Result<String> {
        let command = input
            .get("command")
            .and_then(|value| value.as_str())
            .context("Invalid command")?;

        let dangerous = ["rm -rf /", "sudo", "shutdown", "reboot", "> /dev/"];

        if dangerous.iter().any(|item| command.contains(item)) {
            return Err(anyhow::anyhow!("Error: Dangerous command blocked"));
        }

        let child = match Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(child) => child,
            Err(e) => return Err(anyhow::anyhow!("Error: {}", e)),
        };

        let output_future = child.wait_with_output();

        match timeout(Duration::from_secs(120), output_future).await {
            Ok(Ok(output)) => {
                // trim the stdout and stderr
                let combined = [output.stdout, output.stderr].concat();
                let out_str = String::from_utf8_lossy(&combined);
                let trimmed = out_str.trim();

                if trimmed.is_empty() {
                    Ok("(No output)".to_string())
                } else {
                    // limit output to 50k chars
                    Ok(trimmed.chars().take(50000).collect())
                }
            }
            Ok(Err(e)) => {
                // handle timeout
                Err(anyhow::anyhow!("Error: {}", e))
            }
            Err(_e) => {
                // handle kill_on_drop(true) timeout
                Err(anyhow::anyhow!("Error: Timeout (120s)"))
            }
        }
    }

    fn name(&self) -> Cow<'_, str> {
        "bash".into()
    }

    fn tool_spec(&self) -> ToolSpec {
        ChatCompletionTools::Function(ChatCompletionTool {
            function: FunctionObjectArgs::default()
                .name("bash")
                .description("Run a shell command in the current workspace.")
                .parameters(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                        },
                    },
                    "required": ["command"],
                }))
                .build()
                .expect("Valid Bash Tool Schema"),
        })
    }
}

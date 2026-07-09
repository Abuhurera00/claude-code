use std::{collections::HashMap, collections::HashSet};

use anyhow::{Context, Result};
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionMessageToolCalls, ChatCompletionRequestAssistantMessageArgs,
        ChatCompletionRequestAssistantMessageContent, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessage,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs, FinishReason,
    },
};
use claude_code::tool::{Tool, toolset};
use inquire::Text;

fn get_model() -> anyhow::Result<String> {
    dotenvy::dotenv().ok();
    std::env::var("MODEL").context("MODEL is not set")
}

const SYSTEM: &str = r#"You are a coding agent.
Use bash to inspect and change the workspace. Act first, then report clearly.
"#;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let api_key = std::env::var("API_KEY").context("API_KEY is not set")?;
    let base_url = std::env::var("BASE_URL").context("BASE_URL is not set")?;

    let client = Client::with_config(
        OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(base_url),
    );

    let tools = toolset();
    let mut state = LoopState::new(client.clone(), tools);

    loop {
        let query = Text::new("--- How can I help you today?")
            .prompt()
            .context("An error occurred or user cancelled the input.")?;

        if query.trim() == "exit()" {
            break;
        }

        state.context.push(
            ChatCompletionRequestUserMessageArgs::default()
                .content(query)
                .build()?
                .into(),
        );

        agent_loop(&mut state).await?;

        let Some(final_content) = state.context.last() else {
            continue;
        };

        println!("--- Final response:\n{}", extract_text(final_content));
    }

    Ok(())
}

struct LoopState {
    client: Client<OpenAIConfig>,
    pub context: Vec<ChatCompletionRequestMessage>,
    tools: HashMap<String, Box<dyn Tool>>,
}

impl LoopState {
    fn new(client: Client<OpenAIConfig>, tools: HashMap<String, Box<dyn Tool>>) -> Self {
        Self {
            client,
            context: Vec::new(),
            tools,
        }
    }

    async fn execute_tool_call(
        &mut self,
        content: &[ChatCompletionMessageToolCalls],
    ) -> Vec<ChatCompletionRequestMessage> {
        let mut result = Vec::new();

        for block in content {
            if let ChatCompletionMessageToolCalls::Function(tool_call) = block {
                let output = match serde_json::from_str::<serde_json::Value>(
                    &tool_call.function.arguments,
                ) {
                    Ok(input) => self.execute(&tool_call.function.name, &input).await,
                    Err(error) => format!("Invalid tool arguments: {}", error),
                };

                result.push(ChatCompletionRequestMessage::Tool(
                    ChatCompletionRequestToolMessage {
                        tool_call_id: tool_call.id.clone(),
                        content: output.into(),
                    },
                ));
            }
        }

        result
    }

    async fn execute(&mut self, name: &str, input: &serde_json::Value) -> String {
        let Some(tool) = self.tools.get_mut(name) else {
            return format!("Unknown tool: {name}");
        };

        match tool.invoke(input).await {
            Ok(output) => {
                println!("Tool: {}\nInput: {}\nOutput:\n{}\n", name, input, output);
                output
            }
            Err(error) => {
                println!("Error invoking tool {}: {}", name, error);
                format!("Error invoking tool {}: {}", name, error)
            }
        }
    }
}

fn extract_text(message: &ChatCompletionRequestMessage) -> String {
    match message {
        ChatCompletionRequestMessage::Assistant(message) => match &message.content {
            Some(ChatCompletionRequestAssistantMessageContent::Text(content)) => content.clone(),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

fn normalize_messages(
    messages: &[ChatCompletionRequestMessage],
) -> Vec<ChatCompletionRequestMessage> {
    let mut existing_results = HashSet::new();

    for message in messages {
        if let ChatCompletionRequestMessage::Tool(tool_message) = message {
            existing_results.insert(tool_message.tool_call_id.clone());
        }
    }

    let mut normalized = Vec::new();

    for message in messages {
        normalized.push(message.clone());

        if let ChatCompletionRequestMessage::Assistant(assistant_message) = message
            && let Some(tool_calls) = &assistant_message.tool_calls
        {
            for tool_call in tool_calls {
                if let ChatCompletionMessageToolCalls::Function(function_call) = tool_call
                    && !existing_results.contains(&function_call.id)
                {
                    normalized.push(ChatCompletionRequestMessage::Tool(
                        ChatCompletionRequestToolMessage {
                            tool_call_id: function_call.id.clone(),
                            content: "(cancelled)".to_string().into(),
                        },
                    ));
                }
            }
        }
    }

    normalized
}

#[allow(deprecated)]
async fn run_one_turn(state: &mut LoopState) -> Result<bool> {
    let mut messages = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(SYSTEM)
            .build()?
            .into(),
    ];

    messages.extend(normalize_messages(&state.context));

    let request = CreateChatCompletionRequestArgs::default()
        .model(get_model()?)
        .messages(messages)
        .max_tokens(8000_u32)
        .tools(
            state
                .tools
                .values()
                .map(|tool| tool.tool_spec())
                .collect::<Vec<_>>(),
        )
        .build()?;

    let response = state.client.chat().create(request).await?;

    let choice = response
        .choices
        .first()
        .context("LLM response has no choices")?;

    let stop_reason = choice.finish_reason;
    let response_message = choice.message.clone();

    let mut assistant_message = ChatCompletionRequestAssistantMessageArgs::default();

    if let Some(content) = response_message.content.clone() {
        assistant_message.content(content);
    }

    if let Some(tool_calls) = response_message.tool_calls.clone() {
        assistant_message.tool_calls(tool_calls);
    }

    state.context.push(assistant_message.build()?.into());

    if let Some(stop_reason) = stop_reason
        && !matches!(stop_reason, FinishReason::ToolCalls)
    {
        return Ok(false);
    }

    let Some(tool_calls) = response_message.tool_calls.as_deref() else {
        return Ok(false);
    };

    let tool_result = state.execute_tool_call(tool_calls).await;

    state.context.extend(tool_result);

    Ok(true)
}

async fn agent_loop(state: &mut LoopState) -> Result<()> {
    while run_one_turn(state).await? {}

    Ok(())
}

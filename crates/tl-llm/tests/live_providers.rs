//! Live integration smoke tests. Hit real OpenAI / OpenRouter endpoints
//! to validate the wire shape against the actual provider. Off by default;
//! enable with `cargo test -p tl-llm --features live`. Each test no-ops
//! if its API key is missing, so partial keys don't fail CI.

#![cfg(feature = "live")]

use std::time::Duration;

use serde_json::json;
use tl_llm::{JsonSchema, LlmClient, OpenAiClient, OpenRouterClient};

fn trivial_schema() -> JsonSchema {
    JsonSchema {
        name: "Greeting".into(),
        schema: json!({
            "type": "object",
            "properties": { "answer": { "type": "string" } },
            "required": ["answer"],
            "additionalProperties": false
        }),
    }
}

#[tokio::test]
async fn openai_round_trip() {
    let Ok(client) = OpenAiClient::from_env() else {
        eprintln!("OPENAI_API_KEY not set; skipping");
        return;
    };
    let out = client
        .complete(
            "gpt-4o-mini",
            "Reply with the JSON {\"answer\":\"hello\"} and nothing else.",
            &trivial_schema(),
            Duration::from_secs(15),
        )
        .await
        .expect("response");
    assert!(out.json["answer"].is_string());
    assert!(out.prompt_tokens > 0);
}

#[tokio::test]
async fn openrouter_round_trip() {
    let Ok(client) = OpenRouterClient::from_env() else {
        eprintln!("OPENROUTER_API_KEY not set; skipping");
        return;
    };
    let out = client
        .complete(
            "openai/gpt-4o-mini",
            "Reply with the JSON {\"answer\":\"hello\"} and nothing else.",
            &trivial_schema(),
            Duration::from_secs(15),
        )
        .await
        .expect("response");
    assert!(out.json["answer"].is_string());
}

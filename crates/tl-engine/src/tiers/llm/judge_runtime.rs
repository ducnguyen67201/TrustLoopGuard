use tl_llm::prompts::{authority, hallucination, tone};
use tl_llm::{JudgeKind, LlmError, LlmOutput, LlmRouter};

pub(super) struct JudgeOutcomes {
    pub(super) hallu: JudgeResult,
    pub(super) tone: JudgeResult,
    pub(super) auth: JudgeResult,
}

pub(super) enum JudgeResult {
    Skipped,
    Ok(LlmOutput),
    Err(LlmError),
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_judges(
    router: &LlmRouter,
    tenant: &str,
    do_hallu: bool,
    hallu_prompt: &str,
    do_tone: bool,
    tone_prompt: &str,
    do_auth: bool,
    auth_prompt: &str,
) -> JudgeOutcomes {
    let session = match router.start_session(tenant).await {
        Ok(session) => session,
        Err(error) => {
            tracing::warn!(
                tenant,
                used = error.used,
                limit = error.limit,
                "tenant token budget exceeded"
            );
            return JudgeOutcomes {
                hallu: budget_result(do_hallu),
                tone: budget_result(do_tone),
                auth: budget_result(do_auth),
            };
        }
    };

    let h = async {
        if !do_hallu {
            return JudgeResult::Skipped;
        }
        match session
            .judge(
                JudgeKind::Hallucination,
                hallu_prompt,
                &hallucination::schema(),
            )
            .await
        {
            Ok(output) => JudgeResult::Ok(output),
            Err(error) => JudgeResult::Err(error),
        }
    };

    let t = async {
        if !do_tone {
            return JudgeResult::Skipped;
        }
        match session
            .judge(JudgeKind::Tone, tone_prompt, &tone::schema())
            .await
        {
            Ok(output) => JudgeResult::Ok(output),
            Err(error) => JudgeResult::Err(error),
        }
    };

    let a = async {
        if !do_auth {
            return JudgeResult::Skipped;
        }
        match session
            .judge(JudgeKind::Authority, auth_prompt, &authority::schema())
            .await
        {
            Ok(output) => JudgeResult::Ok(output),
            Err(error) => JudgeResult::Err(error),
        }
    };

    let (hallu, tone, auth) = tokio::join!(h, t, a);
    JudgeOutcomes { hallu, tone, auth }
}

fn budget_result(enabled: bool) -> JudgeResult {
    if enabled {
        JudgeResult::Err(LlmError::BudgetExceeded)
    } else {
        JudgeResult::Skipped
    }
}

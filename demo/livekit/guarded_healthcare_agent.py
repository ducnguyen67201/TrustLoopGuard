"""LiveKit agent example protected by TrustLoopGuard.

This intentionally follows the shape of LiveKit's healthcare example:

    class HealthcareAgent(Agent): ...
    await session.start(agent=HealthcareAgent(...), room=ctx.room)

The important integration point is the ``guardrail`` object. Create it once,
then call it right before a draft reply leaves the agent.
"""

from __future__ import annotations

import logging
import os

from dotenv import load_dotenv
from livekit.agents import Agent, AgentServer, AgentSession, JobContext, cli, inference
from livekit.agents.beta import Instructions
from livekit.plugins import openai, silero

import trustloopguard as trustloop
from trustloopguard import Channel, Decision, GuardLogEvent, OutputGuard, RetryConfig

logger = logging.getLogger("TrustLoopGuardLiveKitDemo")
load_dotenv()

TL_SERVER_URL = os.getenv("TL_SERVER_URL", "http://127.0.0.1:8080")
TL_API_KEY = os.getenv("TL_API_KEY")
TL_AGENT_ID = os.getenv("TL_AGENT_ID", "demo-healthcare-livekit")


async def blocked_reply(decision: Decision) -> str:
    logger.warning("blocked LiveKit reply: %s", decision.reason)
    return "I cannot share that. I can connect you with a human teammate."


async def escalated_reply(decision: Decision) -> str:
    logger.warning("escalated LiveKit reply: %s", decision.reason)
    return "A human teammate should review this before we continue."


def log_guardrail(event: GuardLogEvent) -> None:
    logger.info(
        "trustloopguard decision trace=%s verdict=%s branch=%s latency_ms=%s",
        event.trace_id,
        event.verdict,
        event.branch,
        event.latency_ms,
    )


class HealthcareAgent(Agent):
    def __init__(self, guardrail: OutputGuard) -> None:
        super().__init__(
            instructions=Instructions(
                "You are a healthcare scheduling agent. Be concise. "
                "Do not give medical advice. Escalate out-of-scope requests.",
                text=(
                    "You are a healthcare scheduling agent. Be concise. "
                    "Do not give medical advice. Escalate out-of-scope requests."
                ),
            ),
        )
        self.guardrail = guardrail

    async def on_enter(self) -> None:
        draft = "Hello, I can help schedule or modify an appointment. What can I help with?"
        guarded = await self.guardrail(
            input="call started",
            draft=draft,
            context={
                "livekit": {
                    "room": self.session.room.name if self.session.room else None,
                    "agent": self.__class__.__name__,
                    "source": "agent-draft",
                }
            },
        )
        await self.session.say(guarded, allow_interruptions=True)


server = AgentServer()


@server.rtc_session()
async def entrypoint(ctx: JobContext) -> None:
    guardrail = trustloop.guard(
        agent_id=TL_AGENT_ID,
        base_url=TL_SERVER_URL,
        api_key=TL_API_KEY,
        channel=Channel.voice,
        timeout=0.25,
        retry=RetryConfig(max_attempts=1, total_budget_s=0.25),
        on_block=blocked_reply,
        on_escalate=escalated_reply,
        log=log_guardrail,
    )

    async def on_session_close() -> None:
        await guardrail.aclose()

    ctx.add_shutdown_callback(on_session_close)

    session = AgentSession(
        stt=inference.STT("deepgram/nova-3", language="multi"),
        llm=openai.responses.LLM(),
        tts=inference.TTS("inworld/inworld-tts-1"),
        vad=silero.VAD.load(),
        preemptive_generation=True,
    )

    await session.start(
        agent=HealthcareAgent(guardrail),
        room=ctx.room,
    )


if __name__ == "__main__":
    cli.run_app(server)

"""Smallest possible LiveKit-style TrustLoopGuard integration.

This is the shape we want users to copy:

    import trustloopguard as trustloop
    guardrail = trustloop.guard(agent_id="demo-livekit-agent")
    guarded_reply = await guardrail(input=user_text, draft=agent_draft)
    await session.say(guarded_reply)

No application-level fetch call is needed.
"""

from __future__ import annotations

import os

import trustloopguard as trustloop
from trustloopguard import Channel, Decision, RetryConfig


class LiveKitSupportAgent:
    def __init__(self) -> None:
        self.guardrail = trustloop.guard(
            agent_id=os.getenv("TL_AGENT_ID", "demo-livekit-agent"),
            base_url=os.getenv("TL_SERVER_URL"),
            api_key=os.getenv("TL_API_KEY"),
            channel=Channel.voice,
            timeout=0.25,
            retry=RetryConfig(max_attempts=1, total_budget_s=0.25),
            on_block=self.blocked_reply,
            on_escalate=self.escalated_reply,
        )

    async def before_say(self, *, user_text: str, draft: str) -> str:
        return await self.guardrail(
            input=user_text,
            draft=draft,
        )

    async def blocked_reply(self, decision: Decision) -> str:
        return "I cannot share that. I can connect you with a human teammate."

    async def escalated_reply(self, decision: Decision) -> str:
        return "A human teammate should review this before we continue."

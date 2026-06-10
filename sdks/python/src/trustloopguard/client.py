"""HTTP client for TrustLoopGuard. Mirrors the `Guard.check(draft, ctx)`
plugin contract. Sync and async variants share the same surface."""

from __future__ import annotations

import asyncio
import logging
import random
import time
from typing import Any

import httpx

from urllib.parse import quote

from trustloopguard._generated.types import (
    CheckRequest,
    CreateRunEventRequest,
    CreateRunRequest,
    Decision,
    GuardEvent,
    GuardrailGenerateResponse,
    GuardrailListResponse,
    RunDetail,
    RunEventListResponse,
    RunEventSummary,
    RunListResponse,
    RunStatus,
    RunSummary,
    TraceListResponse,
    UpdateRunRequest,
)
from trustloopguard.errors import (
    Decode,
    SdkError,
    Transport,
    from_response,
    parse_retry_after,
)
from trustloopguard.retry import RetryConfig

# Module-level logger; callers can hook into trustloopguard.* if they want
# our retry decisions in their structured logs.
_logger = logging.getLogger("trustloopguard")


class Client:
    """Synchronous TrustLoopGuard client.

    Args:
        base_url: TrustLoopGuard server URL, e.g. ``"https://api.trustloopguard.dev"``.
        api_key:  Bearer token. Optional in local dev.
        timeout:  Per-request deadline (seconds). Voice callers should pass
                  a tight value (≈0.1s); chat callers can be looser.
        retry:    Retry policy. Defaults to chat-tolerant. Voice callers
                  should pass ``RetryConfig(max_attempts=1)`` to opt out.
    """

    def __init__(
        self,
        base_url: str,
        *,
        api_key: str | None = None,
        timeout: float = 5.0,
        transport: httpx.BaseTransport | None = None,
        retry: RetryConfig | None = None,
    ) -> None:
        self._base_url = base_url.rstrip("/")
        self._api_key = api_key
        self._timeout = timeout
        self._retry = retry or RetryConfig()
        self._http = httpx.Client(
            base_url=self._base_url,
            timeout=timeout,
            transport=transport,
        )

    def check(self, req: CheckRequest, *, timeout: float | None = None) -> Decision:
        # mode="json" coerces Enum / pydantic types into JSON-native scalars
        # so httpx's JSON encoder doesn't trip on Enum instances.
        body = req.model_dump(mode="json", exclude_none=True)
        start = time.monotonic()
        attempt = 0
        while True:
            attempt += 1
            try:
                return self._send_once(body, timeout)
            except SdkError as err:
                elapsed = time.monotonic() - start
                jitter = random.random()
                delay = self._retry.next_delay(attempt, elapsed, err, jitter)
                if delay is None:
                    raise
                _logger.info(
                    "trustloopguard retry: attempt=%d delay=%.3fs error=%s",
                    attempt,
                    delay,
                    err,
                )
                time.sleep(delay)

    def _send_once(
        self, body: dict[str, Any], timeout: float | None
    ) -> Decision:
        try:
            resp = self._http.post(
                "/v1/check",
                json=body,
                headers=self._headers(),
                timeout=timeout if timeout is not None else self._timeout,
            )
        except httpx.RequestError as e:
            raise Transport(str(e)) from e

        if 200 <= resp.status_code < 300:
            try:
                return Decision.model_validate(resp.json())
            except Exception as e:  # noqa: BLE001
                raise Decode(f"failed to parse Decision: {e}") from e

        retry_after = parse_retry_after(resp.headers.get("retry-after"))
        raise from_response(resp.status_code, resp.text, retry_after=retry_after)

    def submit_event(
        self, event: GuardEvent, *, timeout: float | None = None
    ) -> Decision:
        """Submit a full ``GuardEvent`` (sources + provenance) for
        observe-only evidence collection.

        The returned decision's verdict is always ``allow`` with an
        explicit observe-only reason until checker phases ship; do not
        gate behavior on it yet.
        """
        return self._run_with_retry(
            lambda: self._send_json_model(
                "/v1/events",
                method="POST",
                body=event.model_dump(mode="json", exclude_none=True),
                timeout=timeout,
                model=Decision,
            )
        )

    def generate_guardrails(
        self, agent_id: str, *, timeout: float | None = None
    ) -> GuardrailGenerateResponse:
        """Derive a guardrail policy set from an agent's stored ``system_prompt``.

        Each draft is auto-persisted with ``enabled=false`` — review the
        returned set and flip individual policies on via the policies API
        before they take effect at runtime.

        Args:
            agent_id: Agent identifier previously registered via ``POST /v1/agents``.
                Must already have a non-empty ``system_prompt`` on file.

        Raises:
            NotFound: agent is not registered.
            Unprocessable: agent has no ``system_prompt`` set.
            Unavailable: the deployment has no LLM configured (HTTP 503).
        """
        path = f"/v1/agents/{quote(agent_id, safe='')}/guardrails/generate"
        return self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="POST", timeout=timeout, model=GuardrailGenerateResponse
            )
        )

    def list_guardrails(
        self, agent_id: str, *, timeout: float | None = None
    ) -> GuardrailListResponse:
        """List policies owned by ``agent_id``. Empty for unknown agents."""
        path = f"/v1/agents/{quote(agent_id, safe='')}/guardrails"
        return self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="GET", timeout=timeout, model=GuardrailListResponse
            )
        )

    def start_run(
        self, req: CreateRunRequest, *, timeout: float | None = None
    ) -> RunSummary:
        """Create a run grouping for subsequent ``check`` calls."""
        return self._run_with_retry(
            lambda: self._send_json_model(
                "/v1/runs",
                method="POST",
                body=req.model_dump(mode="json", exclude_none=True),
                timeout=timeout,
                model=RunSummary,
            )
        )

    def list_runs(self, *, timeout: float | None = None) -> RunListResponse:
        """List recent runs for the authenticated workspace."""
        return self._run_with_retry(
            lambda: self._send_get_or_post(
                "/v1/runs", method="GET", timeout=timeout, model=RunListResponse
            )
        )

    def get_run(self, run_id: str, *, timeout: float | None = None) -> RunDetail:
        """Fetch a run and its recent traces."""
        path = f"/v1/runs/{quote(run_id, safe='')}"
        return self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="GET", timeout=timeout, model=RunDetail
            )
        )

    def update_run(
        self, run_id: str, req: UpdateRunRequest, *, timeout: float | None = None
    ) -> RunSummary:
        """Update a run's status, metadata, or end timestamp."""
        path = f"/v1/runs/{quote(run_id, safe='')}"
        return self._run_with_retry(
            lambda: self._send_json_model(
                path,
                method="PATCH",
                body=req.model_dump(mode="json", exclude_none=True),
                timeout=timeout,
                model=RunSummary,
            )
        )

    def finish_run(
        self,
        run_id: str,
        status: RunStatus = RunStatus.completed,
        *,
        timeout: float | None = None,
    ) -> RunSummary:
        """Mark a run completed, failed, or canceled."""
        return self.update_run(
            run_id, UpdateRunRequest(status=status), timeout=timeout
        )

    def create_run_event(
        self,
        run_id: str,
        req: CreateRunEventRequest,
        *,
        timeout: float | None = None,
    ) -> RunEventSummary:
        """Append an event to a run timeline."""
        path = f"/v1/runs/{quote(run_id, safe='')}/events"
        return self._run_with_retry(
            lambda: self._send_json_model(
                path,
                method="POST",
                body=req.model_dump(mode="json", exclude_none=True),
                timeout=timeout,
                model=RunEventSummary,
            )
        )

    def list_run_events(
        self, run_id: str, *, timeout: float | None = None
    ) -> RunEventListResponse:
        """List events attached to a run timeline."""
        path = f"/v1/runs/{quote(run_id, safe='')}/events"
        return self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="GET", timeout=timeout, model=RunEventListResponse
            )
        )

    def list_run_traces(
        self, run_id: str, *, timeout: float | None = None
    ) -> TraceListResponse:
        """List traces grouped under a run."""
        path = f"/v1/runs/{quote(run_id, safe='')}/traces"
        return self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="GET", timeout=timeout, model=TraceListResponse
            )
        )

    def _run_with_retry(self, send: Any) -> Any:
        start = time.monotonic()
        attempt = 0
        while True:
            attempt += 1
            try:
                return send()
            except SdkError as err:
                elapsed = time.monotonic() - start
                jitter = random.random()
                delay = self._retry.next_delay(attempt, elapsed, err, jitter)
                if delay is None:
                    raise
                _logger.info(
                    "trustloopguard retry: attempt=%d delay=%.3fs error=%s",
                    attempt,
                    delay,
                    err,
                )
                time.sleep(delay)

    def _send_get_or_post(
        self,
        path: str,
        *,
        method: str,
        timeout: float | None,
        model: Any,
    ) -> Any:
        try:
            resp = self._http.request(
                method,
                path,
                headers=self._headers(),
                timeout=timeout if timeout is not None else self._timeout,
            )
        except httpx.RequestError as e:
            raise Transport(str(e)) from e

        if 200 <= resp.status_code < 300:
            try:
                return model.model_validate(resp.json())
            except Exception as e:  # noqa: BLE001
                raise Decode(f"failed to parse {model.__name__}: {e}") from e

        retry_after = parse_retry_after(resp.headers.get("retry-after"))
        raise from_response(resp.status_code, resp.text, retry_after=retry_after)

    def _send_json_model(
        self,
        path: str,
        *,
        method: str,
        body: dict[str, Any],
        timeout: float | None,
        model: Any,
    ) -> Any:
        try:
            resp = self._http.request(
                method,
                path,
                json=body,
                headers=self._headers(),
                timeout=timeout if timeout is not None else self._timeout,
            )
        except httpx.RequestError as e:
            raise Transport(str(e)) from e

        if 200 <= resp.status_code < 300:
            try:
                return model.model_validate(resp.json())
            except Exception as e:  # noqa: BLE001
                raise Decode(f"failed to parse {model.__name__}: {e}") from e

        retry_after = parse_retry_after(resp.headers.get("retry-after"))
        raise from_response(resp.status_code, resp.text, retry_after=retry_after)

    def close(self) -> None:
        self._http.close()

    def __enter__(self) -> "Client":
        return self

    def __exit__(self, *_: Any) -> None:
        self.close()

    def _headers(self) -> dict[str, str]:
        h = {"content-type": "application/json"}
        if self._api_key:
            h["authorization"] = f"Bearer {self._api_key}"
        return h


class AsyncClient:
    """Async TrustLoopGuard client. Same surface as ``Client`` but awaitable."""

    def __init__(
        self,
        base_url: str,
        *,
        api_key: str | None = None,
        timeout: float = 5.0,
        transport: httpx.AsyncBaseTransport | None = None,
        retry: RetryConfig | None = None,
    ) -> None:
        self._base_url = base_url.rstrip("/")
        self._api_key = api_key
        self._timeout = timeout
        self._retry = retry or RetryConfig()
        self._http = httpx.AsyncClient(
            base_url=self._base_url,
            timeout=timeout,
            transport=transport,
        )

    async def check(self, req: CheckRequest, *, timeout: float | None = None) -> Decision:
        body = req.model_dump(mode="json", exclude_none=True)
        start = time.monotonic()
        attempt = 0
        while True:
            attempt += 1
            try:
                return await self._send_once(body, timeout)
            except SdkError as err:
                elapsed = time.monotonic() - start
                jitter = random.random()
                delay = self._retry.next_delay(attempt, elapsed, err, jitter)
                if delay is None:
                    raise
                _logger.info(
                    "trustloopguard retry: attempt=%d delay=%.3fs error=%s",
                    attempt,
                    delay,
                    err,
                )
                await asyncio.sleep(delay)

    async def _send_once(
        self, body: dict[str, Any], timeout: float | None
    ) -> Decision:
        try:
            resp = await self._http.post(
                "/v1/check",
                json=body,
                headers=self._headers(),
                timeout=timeout if timeout is not None else self._timeout,
            )
        except httpx.RequestError as e:
            raise Transport(str(e)) from e

        if 200 <= resp.status_code < 300:
            try:
                return Decision.model_validate(resp.json())
            except Exception as e:  # noqa: BLE001
                raise Decode(f"failed to parse Decision: {e}") from e

        retry_after = parse_retry_after(resp.headers.get("retry-after"))
        raise from_response(resp.status_code, resp.text, retry_after=retry_after)

    async def submit_event(
        self, event: GuardEvent, *, timeout: float | None = None
    ) -> Decision:
        """Async variant of ``Client.submit_event``."""
        return await self._run_with_retry(
            lambda: self._send_json_model(
                "/v1/events",
                method="POST",
                body=event.model_dump(mode="json", exclude_none=True),
                timeout=timeout,
                model=Decision,
            )
        )

    async def generate_guardrails(
        self, agent_id: str, *, timeout: float | None = None
    ) -> GuardrailGenerateResponse:
        """Async variant of ``Client.generate_guardrails``."""
        path = f"/v1/agents/{quote(agent_id, safe='')}/guardrails/generate"
        return await self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="POST", timeout=timeout, model=GuardrailGenerateResponse
            )
        )

    async def list_guardrails(
        self, agent_id: str, *, timeout: float | None = None
    ) -> GuardrailListResponse:
        """Async variant of ``Client.list_guardrails``."""
        path = f"/v1/agents/{quote(agent_id, safe='')}/guardrails"
        return await self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="GET", timeout=timeout, model=GuardrailListResponse
            )
        )

    async def start_run(
        self, req: CreateRunRequest, *, timeout: float | None = None
    ) -> RunSummary:
        """Async variant of ``Client.start_run``."""
        return await self._run_with_retry(
            lambda: self._send_json_model(
                "/v1/runs",
                method="POST",
                body=req.model_dump(mode="json", exclude_none=True),
                timeout=timeout,
                model=RunSummary,
            )
        )

    async def list_runs(self, *, timeout: float | None = None) -> RunListResponse:
        """Async variant of ``Client.list_runs``."""
        return await self._run_with_retry(
            lambda: self._send_get_or_post(
                "/v1/runs", method="GET", timeout=timeout, model=RunListResponse
            )
        )

    async def get_run(self, run_id: str, *, timeout: float | None = None) -> RunDetail:
        """Async variant of ``Client.get_run``."""
        path = f"/v1/runs/{quote(run_id, safe='')}"
        return await self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="GET", timeout=timeout, model=RunDetail
            )
        )

    async def update_run(
        self, run_id: str, req: UpdateRunRequest, *, timeout: float | None = None
    ) -> RunSummary:
        """Async variant of ``Client.update_run``."""
        path = f"/v1/runs/{quote(run_id, safe='')}"
        return await self._run_with_retry(
            lambda: self._send_json_model(
                path,
                method="PATCH",
                body=req.model_dump(mode="json", exclude_none=True),
                timeout=timeout,
                model=RunSummary,
            )
        )

    async def finish_run(
        self,
        run_id: str,
        status: RunStatus = RunStatus.completed,
        *,
        timeout: float | None = None,
    ) -> RunSummary:
        """Async variant of ``Client.finish_run``."""
        return await self.update_run(
            run_id, UpdateRunRequest(status=status), timeout=timeout
        )

    async def create_run_event(
        self,
        run_id: str,
        req: CreateRunEventRequest,
        *,
        timeout: float | None = None,
    ) -> RunEventSummary:
        """Async variant of ``Client.create_run_event``."""
        path = f"/v1/runs/{quote(run_id, safe='')}/events"
        return await self._run_with_retry(
            lambda: self._send_json_model(
                path,
                method="POST",
                body=req.model_dump(mode="json", exclude_none=True),
                timeout=timeout,
                model=RunEventSummary,
            )
        )

    async def list_run_events(
        self, run_id: str, *, timeout: float | None = None
    ) -> RunEventListResponse:
        """Async variant of ``Client.list_run_events``."""
        path = f"/v1/runs/{quote(run_id, safe='')}/events"
        return await self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="GET", timeout=timeout, model=RunEventListResponse
            )
        )

    async def list_run_traces(
        self, run_id: str, *, timeout: float | None = None
    ) -> TraceListResponse:
        """Async variant of ``Client.list_run_traces``."""
        path = f"/v1/runs/{quote(run_id, safe='')}/traces"
        return await self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="GET", timeout=timeout, model=TraceListResponse
            )
        )

    async def _run_with_retry(self, send: Any) -> Any:
        start = time.monotonic()
        attempt = 0
        while True:
            attempt += 1
            try:
                return await send()
            except SdkError as err:
                elapsed = time.monotonic() - start
                jitter = random.random()
                delay = self._retry.next_delay(attempt, elapsed, err, jitter)
                if delay is None:
                    raise
                _logger.info(
                    "trustloopguard retry: attempt=%d delay=%.3fs error=%s",
                    attempt,
                    delay,
                    err,
                )
                await asyncio.sleep(delay)

    async def _send_get_or_post(
        self,
        path: str,
        *,
        method: str,
        timeout: float | None,
        model: Any,
    ) -> Any:
        try:
            resp = await self._http.request(
                method,
                path,
                headers=self._headers(),
                timeout=timeout if timeout is not None else self._timeout,
            )
        except httpx.RequestError as e:
            raise Transport(str(e)) from e

        if 200 <= resp.status_code < 300:
            try:
                return model.model_validate(resp.json())
            except Exception as e:  # noqa: BLE001
                raise Decode(f"failed to parse {model.__name__}: {e}") from e

        retry_after = parse_retry_after(resp.headers.get("retry-after"))
        raise from_response(resp.status_code, resp.text, retry_after=retry_after)

    async def _send_json_model(
        self,
        path: str,
        *,
        method: str,
        body: dict[str, Any],
        timeout: float | None,
        model: Any,
    ) -> Any:
        try:
            resp = await self._http.request(
                method,
                path,
                json=body,
                headers=self._headers(),
                timeout=timeout if timeout is not None else self._timeout,
            )
        except httpx.RequestError as e:
            raise Transport(str(e)) from e

        if 200 <= resp.status_code < 300:
            try:
                return model.model_validate(resp.json())
            except Exception as e:  # noqa: BLE001
                raise Decode(f"failed to parse {model.__name__}: {e}") from e

        retry_after = parse_retry_after(resp.headers.get("retry-after"))
        raise from_response(resp.status_code, resp.text, retry_after=retry_after)

    async def aclose(self) -> None:
        await self._http.aclose()

    async def __aenter__(self) -> "AsyncClient":
        return self

    async def __aexit__(self, *_: Any) -> None:
        await self.aclose()

    def _headers(self) -> dict[str, str]:
        h = {"content-type": "application/json"}
        if self._api_key:
            h["authorization"] = f"Bearer {self._api_key}"
        return h

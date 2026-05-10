"""HTTP client for TrustLoopGuard. Mirrors the `Guard.check(draft, ctx)`
plugin contract. Sync and async variants share the same surface."""

from __future__ import annotations

from typing import Any

import httpx

from trustloopguard._generated.types import CheckRequest, Decision


class Client:
    """Synchronous TrustLoopGuard client.

    Args:
        base_url: TrustLoopGuard server URL, e.g. ``"https://api.trustloopguard.dev"``.
        api_key:  Bearer token. Optional in local dev.
        timeout:  Per-request deadline (seconds). Voice callers should pass
                  a tight value (≈0.1s); chat callers can be looser.
    """

    def __init__(
        self,
        base_url: str,
        *,
        api_key: str | None = None,
        timeout: float = 5.0,
        transport: httpx.BaseTransport | None = None,
    ) -> None:
        self._base_url = base_url.rstrip("/")
        self._api_key = api_key
        self._timeout = timeout
        self._http = httpx.Client(
            base_url=self._base_url,
            timeout=timeout,
            transport=transport,
        )

    def check(self, req: CheckRequest, *, timeout: float | None = None) -> Decision:
        headers = self._headers()
        # mode="json" coerces Enum / pydantic types into JSON-native scalars
        # so httpx's JSON encoder doesn't trip on Enum instances.
        body = req.model_dump(mode="json", exclude_none=True)
        resp = self._http.post(
            "/v1/check",
            json=body,
            headers=headers,
            timeout=timeout if timeout is not None else self._timeout,
        )
        resp.raise_for_status()
        return Decision.model_validate(resp.json())

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
    ) -> None:
        self._base_url = base_url.rstrip("/")
        self._api_key = api_key
        self._timeout = timeout
        self._http = httpx.AsyncClient(
            base_url=self._base_url,
            timeout=timeout,
            transport=transport,
        )

    async def check(self, req: CheckRequest, *, timeout: float | None = None) -> Decision:
        headers = self._headers()
        # mode="json" coerces Enum / pydantic types into JSON-native scalars
        # so httpx's JSON encoder doesn't trip on Enum instances.
        body = req.model_dump(mode="json", exclude_none=True)
        resp = await self._http.post(
            "/v1/check",
            json=body,
            headers=headers,
            timeout=timeout if timeout is not None else self._timeout,
        )
        resp.raise_for_status()
        return Decision.model_validate(resp.json())

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

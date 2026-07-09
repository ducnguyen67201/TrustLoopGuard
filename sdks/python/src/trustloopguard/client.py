"""HTTP client for TrustLoopGuard's GuardEvent runtime contract.

Sync and async variants share the same surface.
"""

from __future__ import annotations

import asyncio
import contextvars
import logging
import random
import time
from typing import Any, Callable, Generic, TypeVar

import httpx

from urllib.parse import quote, urlencode

from trustloopguard._generated.types import (
    Action,
    AgenticPaymentAuthorizationResponse,
    AgenticPaymentAuthorizeRequest,
    AgenticPaymentCommitRequest,
    AgenticPaymentRecord,
    AgenticPaymentRollbackRequest,
    CommitFinancialActionRequest,
    CommitFinancialActionResponse,
    CounterpartyRef,
    CreateFinancialActionRequest,
    CreateFinancialExecutionConnectorRequest,
    CreateFinancialExecutionConnectorResponse,
    CreateFinancialMandateRequest,
    CreateFinancialObservationReviewRequest,
    CreateFinancialPolicyRequest,
    CreateRunEventRequest,
    CreateRunRequest,
    Decision,
    EventKind,
    EvidenceRef,
    FinancialAction,
    FinancialActionDecisionReceipt,
    FinancialActionKind,
    FinancialActionListResponse,
    FinancialActionOutcome,
    FinancialActionRecord,
    FinancialApprovalRequestListResponse,
    FinancialExecutionConnector,
    FinancialExecutionConnectorListResponse,
    FinancialMandate,
    FinancialMandateListResponse,
    FinancialOutcomeListResponse,
    FinancialObservationReview,
    FinancialObservationReviewListResponse,
    FinancialObservationSummaryResponse,
    FinancialPolicyListResponse,
    FinancialPolicyRecord,
    FinancialRail,
    FinancialReceipt,
    GuardEvent,
    GuardrailGenerateResponse,
    GuardrailListResponse,
    MandateRef,
    MoneyAmount,
    PolicyDocument,
    PolicyFamily,
    PolicyListResponse,
    Principal,
    RunDetail,
    RunEventListResponse,
    RunEventSummary,
    RunKind,
    RunListResponse,
    RunStatus,
    RunSummary,
    SideEffectClass,
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
_run_context: contextvars.ContextVar[dict[str, str]] = contextvars.ContextVar(
    "trustloopguard_run_context", default={}
)
InputT = TypeVar("InputT")
FactsT = TypeVar("FactsT")


def _merge_context(event: GuardEvent) -> GuardEvent:
    ctx = _run_context.get()
    if not ctx:
        return event
    event = event.model_copy(deep=True)
    if event.principal.run_id is None and "run_id" in ctx:
        event.principal.run_id = ctx["run_id"]
    if (
        event.principal.run_event_id is None
        and event.principal.run_id == ctx.get("run_id")
        and "run_event_id" in ctx
    ):
        event.principal.run_event_id = ctx["run_event_id"]
    return event


def _run_only_context(run_id: str) -> dict[str, str]:
    ctx = {**_run_context.get(), "run_id": run_id}
    ctx.pop("run_event_id", None)
    return ctx


def _run_request(
    *,
    agent_id: str,
    kind: RunKind,
    external_id: str | None,
    metadata: dict[str, Any] | None,
    input_summary: str | None,
) -> CreateRunRequest:
    body = dict(metadata or {})
    if input_summary:
        body["input_summary"] = input_summary
    return CreateRunRequest(
        agent_id=agent_id,
        kind=kind,
        external_id=external_id,
        metadata=body,
    )


class FinancialOperation(Generic[InputT, FactsT]):
    def __init__(
        self,
        client: "Client",
        *,
        operation: str,
        kind: FinancialActionKind,
        principal_id: str,
        rail: FinancialRail,
        amount: Callable[[InputT, FactsT | None], MoneyAmount],
        idempotency_key: Callable[[InputT, FactsT | None], str],
        counterparty: Callable[[InputT, FactsT | None], CounterpartyRef | None] | None = None,
        mandate: Callable[[InputT, FactsT | None], MandateRef | None] | None = None,
        memo: Callable[[InputT, FactsT | None], str | None] | None = None,
        metadata: Callable[[InputT, FactsT | None], dict[str, Any] | None] | None = None,
        evidence: Callable[[InputT, FactsT | None], list[EvidenceRef]] | None = None,
        execute: bool = False,
    ) -> None:
        self._client = client
        self._operation = _clean_financial_operation_field("operation", operation)
        self._kind = kind
        self._principal_id = _clean_financial_operation_field("principal_id", principal_id)
        self._rail = rail
        self._amount = amount
        self._idempotency_key = idempotency_key
        self._counterparty = counterparty
        self._mandate = mandate
        self._memo = memo
        self._metadata = metadata
        self._evidence = evidence
        self._execute = execute

    def build_request(
        self,
        input: InputT,
        facts: FactsT | None = None,
        *,
        execute: bool | None = None,
    ) -> CreateFinancialActionRequest:
        return _build_financial_operation_request(
            input,
            facts,
            operation=self._operation,
            kind=self._kind,
            principal_id=self._principal_id,
            rail=self._rail,
            amount=self._amount,
            idempotency_key=self._idempotency_key,
            counterparty=self._counterparty,
            mandate=self._mandate,
            memo=self._memo,
            metadata=self._metadata,
            evidence=self._evidence,
            execute=self._execute if execute is None else execute,
        )

    def verify(
        self,
        input: InputT,
        facts: FactsT | None = None,
        *,
        execute: bool | None = None,
        timeout: float | None = None,
    ) -> FinancialActionRecord:
        return self._client.verify_action(
            self.build_request(input, facts, execute=execute), timeout=timeout
        )


class AsyncFinancialOperation(Generic[InputT, FactsT]):
    def __init__(
        self,
        client: "AsyncClient",
        *,
        operation: str,
        kind: FinancialActionKind,
        principal_id: str,
        rail: FinancialRail,
        amount: Callable[[InputT, FactsT | None], MoneyAmount],
        idempotency_key: Callable[[InputT, FactsT | None], str],
        counterparty: Callable[[InputT, FactsT | None], CounterpartyRef | None] | None = None,
        mandate: Callable[[InputT, FactsT | None], MandateRef | None] | None = None,
        memo: Callable[[InputT, FactsT | None], str | None] | None = None,
        metadata: Callable[[InputT, FactsT | None], dict[str, Any] | None] | None = None,
        evidence: Callable[[InputT, FactsT | None], list[EvidenceRef]] | None = None,
        execute: bool = False,
    ) -> None:
        self._client = client
        self._operation = _clean_financial_operation_field("operation", operation)
        self._kind = kind
        self._principal_id = _clean_financial_operation_field("principal_id", principal_id)
        self._rail = rail
        self._amount = amount
        self._idempotency_key = idempotency_key
        self._counterparty = counterparty
        self._mandate = mandate
        self._memo = memo
        self._metadata = metadata
        self._evidence = evidence
        self._execute = execute

    def build_request(
        self,
        input: InputT,
        facts: FactsT | None = None,
        *,
        execute: bool | None = None,
    ) -> CreateFinancialActionRequest:
        return _build_financial_operation_request(
            input,
            facts,
            operation=self._operation,
            kind=self._kind,
            principal_id=self._principal_id,
            rail=self._rail,
            amount=self._amount,
            idempotency_key=self._idempotency_key,
            counterparty=self._counterparty,
            mandate=self._mandate,
            memo=self._memo,
            metadata=self._metadata,
            evidence=self._evidence,
            execute=self._execute if execute is None else execute,
        )

    async def verify(
        self,
        input: InputT,
        facts: FactsT | None = None,
        *,
        execute: bool | None = None,
        timeout: float | None = None,
    ) -> FinancialActionRecord:
        return await self._client.verify_action(
            self.build_request(input, facts, execute=execute), timeout=timeout
        )


def _build_financial_operation_request(
    input: InputT,
    facts: FactsT | None,
    *,
    operation: str,
    kind: FinancialActionKind,
    principal_id: str,
    rail: FinancialRail,
    amount: Callable[[InputT, FactsT | None], MoneyAmount],
    idempotency_key: Callable[[InputT, FactsT | None], str],
    counterparty: Callable[[InputT, FactsT | None], CounterpartyRef | None] | None,
    mandate: Callable[[InputT, FactsT | None], MandateRef | None] | None,
    memo: Callable[[InputT, FactsT | None], str | None] | None,
    metadata: Callable[[InputT, FactsT | None], dict[str, Any] | None] | None,
    evidence: Callable[[InputT, FactsT | None], list[EvidenceRef]] | None,
    execute: bool,
) -> CreateFinancialActionRequest:
    return CreateFinancialActionRequest(
        idempotency_key=_clean_financial_operation_field(
            "idempotency_key", idempotency_key(input, facts)
        ),
        execute=execute,
        action=FinancialAction(
            kind=kind,
            operation=operation,
            principal_id=principal_id,
            amount=amount(input, facts),
            counterparty=counterparty(input, facts) if counterparty else None,
            rail=rail,
            mandate=mandate(input, facts) if mandate else None,
            memo=memo(input, facts) if memo else None,
            metadata=metadata(input, facts) if metadata else {},
        ),
        evidence=evidence(input, facts) if evidence else [],
    )


def _clean_financial_operation_field(name: str, value: str) -> str:
    trimmed = value.strip()
    if not trimmed:
        raise ValueError(f"{name} must not be empty")
    return trimmed


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

    def submit_event(
        self, event: GuardEvent, *, timeout: float | None = None
    ) -> Decision:
        """Submit a full ``GuardEvent`` (sources + provenance) for a runtime decision."""
        event = _merge_context(event)
        return self._run_with_retry(
            lambda: self._send_json_model(
                "/v1/events",
                method="POST",
                body=event.model_dump(mode="json", exclude_none=True),
                timeout=timeout,
                model=Decision,
            )
        )

    def run(
        self,
        *,
        agent_id: str,
        kind: RunKind = RunKind.other,
        external_id: str | None = None,
        metadata: dict[str, Any] | None = None,
        input_summary: str | None = None,
        finish_on_error: bool = True,
    ) -> "_RunContext":
        return _RunContext(
            self,
            _run_request(
                agent_id=agent_id,
                kind=kind,
                external_id=external_id,
                metadata=metadata,
                input_summary=input_summary,
            ),
            finish_on_error,
        )

    def guard_tool_call(
        self,
        *,
        agent_id: str,
        operation: str,
        parameters: dict[str, Any] | None = None,
        side_effect: str | SideEffectClass | None = None,
        sources: list[Any] | None = None,
        provenance: dict[str, list[str]] | None = None,
        context: dict[str, Any] | None = None,
    ) -> Decision:
        return self.submit_event(
            GuardEvent.model_validate(
                {
                    "kind": EventKind.tool_call_proposed,
                    "principal": {
                        "workspace_id": "",
                        "environment_id": "",
                        "agent_id": agent_id,
                    },
                    "action": {
                        "operation": operation,
                        "parameters": parameters or {},
                        **({"side_effect": side_effect} if side_effect else {}),
                    },
                    "sources": sources or [],
                    "provenance": provenance or {},
                    "context": context,
                }
            )
        )

    def verify_action(
        self, req: CreateFinancialActionRequest, *, timeout: float | None = None
    ) -> FinancialActionRecord:
        """Create or verify a typed financial action."""
        return self._run_with_retry(
            lambda: self._send_json_model(
                "/v1/financial/actions",
                method="POST",
                body=req.model_dump(mode="json", exclude_none=True),
                timeout=timeout,
                model=FinancialActionRecord,
            )
        )

    def guard_payment(
        self, req: CreateFinancialActionRequest, *, timeout: float | None = None
    ) -> FinancialActionRecord:
        """Alias for payment/refund callers that submit financial actions."""
        return self.verify_action(req, timeout=timeout)

    def financial_operation(
        self,
        *,
        operation: str,
        kind: FinancialActionKind,
        principal_id: str,
        rail: FinancialRail,
        amount: Callable[[InputT, FactsT | None], MoneyAmount],
        idempotency_key: Callable[[InputT, FactsT | None], str],
        counterparty: Callable[[InputT, FactsT | None], CounterpartyRef | None] | None = None,
        mandate: Callable[[InputT, FactsT | None], MandateRef | None] | None = None,
        memo: Callable[[InputT, FactsT | None], str | None] | None = None,
        metadata: Callable[[InputT, FactsT | None], dict[str, Any] | None] | None = None,
        evidence: Callable[[InputT, FactsT | None], list[EvidenceRef]] | None = None,
        execute: bool = False,
    ) -> FinancialOperation[InputT, FactsT]:
        return FinancialOperation(
            self,
            operation=operation,
            kind=kind,
            principal_id=principal_id,
            rail=rail,
            amount=amount,
            idempotency_key=idempotency_key,
            counterparty=counterparty,
            mandate=mandate,
            memo=memo,
            metadata=metadata,
            evidence=evidence,
            execute=execute,
        )

    def get_financial_action(
        self, action_id: str, *, timeout: float | None = None
    ) -> FinancialActionRecord:
        path = f"/v1/financial/actions/{quote(action_id, safe='')}"
        return self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="GET", timeout=timeout, model=FinancialActionRecord
            )
        )

    def list_financial_actions(
        self, *, timeout: float | None = None
    ) -> FinancialActionListResponse:
        return self._run_with_retry(
            lambda: self._send_get_or_post(
                "/v1/financial/actions",
                method="GET",
                timeout=timeout,
                model=FinancialActionListResponse,
            )
        )

    def authorize_agentic_payment(
        self, req: AgenticPaymentAuthorizeRequest, *, timeout: float | None = None
    ) -> AgenticPaymentAuthorizationResponse:
        return self._run_with_retry(
            lambda: self._send_json_model(
                "/v1/financial/agentic-payments/authorize",
                method="POST",
                body=req.model_dump(mode="json", exclude_none=True),
                timeout=timeout,
                model=AgenticPaymentAuthorizationResponse,
            )
        )

    def get_agentic_payment(
        self, action_id: str, *, timeout: float | None = None
    ) -> AgenticPaymentRecord:
        path = f"/v1/financial/agentic-payments/{quote(action_id, safe='')}"
        return self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="GET", timeout=timeout, model=AgenticPaymentRecord
            )
        )

    def commit_agentic_payment(
        self,
        action_id: str,
        req: AgenticPaymentCommitRequest,
        *,
        timeout: float | None = None,
    ) -> AgenticPaymentRecord:
        path = f"/v1/financial/agentic-payments/{quote(action_id, safe='')}/commit"
        return self._send_json_model(
            path,
            method="POST",
            body=req.model_dump(mode="json", exclude_none=True),
            timeout=timeout,
            model=AgenticPaymentRecord,
        )

    def rollback_agentic_payment(
        self,
        action_id: str,
        req: AgenticPaymentRollbackRequest,
        *,
        timeout: float | None = None,
    ) -> AgenticPaymentRecord:
        path = f"/v1/financial/agentic-payments/{quote(action_id, safe='')}/rollback"
        return self._send_json_model(
            path,
            method="POST",
            body=req.model_dump(mode="json", exclude_none=True),
            timeout=timeout,
            model=AgenticPaymentRecord,
        )

    def get_agentic_payment_receipt(
        self, action_id: str, *, timeout: float | None = None
    ) -> FinancialReceipt:
        path = f"/v1/financial/agentic-payments/{quote(action_id, safe='')}/receipt"
        return self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="GET", timeout=timeout, model=FinancialReceipt
            )
        )

    def commit_financial_action(
        self,
        action_id: str,
        req: CommitFinancialActionRequest,
        *,
        timeout: float | None = None,
    ) -> CommitFinancialActionResponse:
        path = f"/v1/financial/actions/{quote(action_id, safe='')}/commit"
        return self._send_json_model(
            path,
            method="POST",
            body=req.model_dump(mode="json", exclude_none=True),
            timeout=timeout,
            model=CommitFinancialActionResponse,
        )

    def create_financial_execution_connector(
        self,
        req: CreateFinancialExecutionConnectorRequest,
        *,
        timeout: float | None = None,
    ) -> CreateFinancialExecutionConnectorResponse:
        return self._send_json_model(
            "/v1/financial/execution-connectors",
            method="POST",
            body=req.model_dump(mode="json", exclude_none=True),
            timeout=timeout,
            model=CreateFinancialExecutionConnectorResponse,
        )

    def list_financial_execution_connectors(
        self, *, timeout: float | None = None
    ) -> FinancialExecutionConnectorListResponse:
        return self._run_with_retry(
            lambda: self._send_get_or_post(
                "/v1/financial/execution-connectors",
                method="GET",
                timeout=timeout,
                model=FinancialExecutionConnectorListResponse,
            )
        )

    def revoke_financial_execution_connector(
        self, connector_id: str, *, timeout: float | None = None
    ) -> FinancialExecutionConnector:
        path = (
            f"/v1/financial/execution-connectors/{quote(connector_id, safe='')}/revoke"
        )
        return self._send_get_or_post(
            path, method="POST", timeout=timeout, model=FinancialExecutionConnector
        )

    def financial_observation_summary(
        self, start: str, end: str, *, timeout: float | None = None
    ) -> FinancialObservationSummaryResponse:
        path = f"/v1/financial/observations/summary?{urlencode({'start': start, 'end': end})}"
        return self._run_with_retry(
            lambda: self._send_get_or_post(
                path,
                method="GET",
                timeout=timeout,
                model=FinancialObservationSummaryResponse,
            )
        )

    def create_financial_observation_review(
        self,
        action_id: str,
        req: CreateFinancialObservationReviewRequest,
        *,
        timeout: float | None = None,
    ) -> FinancialObservationReview:
        path = (
            f"/v1/financial/actions/{quote(action_id, safe='')}/observation-reviews"
        )
        return self._send_json_model(
            path,
            method="POST",
            body=req.model_dump(mode="json", exclude_none=True),
            timeout=timeout,
            model=FinancialObservationReview,
        )

    def list_financial_observation_reviews(
        self, action_id: str, *, timeout: float | None = None
    ) -> FinancialObservationReviewListResponse:
        path = (
            f"/v1/financial/actions/{quote(action_id, safe='')}/observation-reviews"
        )
        return self._run_with_retry(
            lambda: self._send_get_or_post(
                path,
                method="GET",
                timeout=timeout,
                model=FinancialObservationReviewListResponse,
            )
        )

    def create_financial_policy(
        self, req: CreateFinancialPolicyRequest, *, timeout: float | None = None
    ) -> FinancialPolicyRecord:
        """Create or update a financial spending control."""
        return self._send_json_model(
            "/v1/financial/policies",
            method="POST",
            body=req.model_dump(mode="json", exclude_none=True),
            timeout=timeout,
            model=FinancialPolicyRecord,
        )

    def list_financial_policies(
        self, *, timeout: float | None = None
    ) -> FinancialPolicyListResponse:
        return self._run_with_retry(
            lambda: self._send_get_or_post(
                "/v1/financial/policies",
                method="GET",
                timeout=timeout,
                model=FinancialPolicyListResponse,
            )
        )

    def list_policies(
        self,
        *,
        family: PolicyFamily | None = None,
        timeout: float | None = None,
    ) -> PolicyListResponse:
        path = "/v1/policies"
        if family is not None:
            path = f"{path}?family={quote(family.value, safe='')}"
        return self._run_with_retry(
            lambda: self._send_get_or_post(
                path,
                method="GET",
                timeout=timeout,
                model=PolicyListResponse,
            )
        )

    def get_policy(
        self, policy_id: str, *, timeout: float | None = None
    ) -> PolicyDocument:
        path = f"/v1/policies/{quote(policy_id, safe='')}"
        return self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="GET", timeout=timeout, model=PolicyDocument
            )
        )

    def upsert_policy(
        self, source_yaml: str, *, timeout: float | None = None
    ) -> PolicyDocument:
        return self._run_with_retry(
            lambda: self._send_text_model(
                "/v1/policies",
                method="POST",
                body=source_yaml,
                content_type="application/yaml",
                timeout=timeout,
                model=PolicyDocument,
            )
        )

    def create_mandate(
        self, req: CreateFinancialMandateRequest, *, timeout: float | None = None
    ) -> FinancialMandate:
        return self._send_json_model(
            "/v1/financial/mandates",
            method="POST",
            body=req.model_dump(mode="json", exclude_none=True),
            timeout=timeout,
            model=FinancialMandate,
        )

    def list_mandates(
        self, *, timeout: float | None = None
    ) -> FinancialMandateListResponse:
        return self._run_with_retry(
            lambda: self._send_get_or_post(
                "/v1/financial/mandates",
                method="GET",
                timeout=timeout,
                model=FinancialMandateListResponse,
            )
        )

    def list_approval_requests(
        self, *, timeout: float | None = None
    ) -> FinancialApprovalRequestListResponse:
        return self._run_with_retry(
            lambda: self._send_get_or_post(
                "/v1/financial/approval-requests",
                method="GET",
                timeout=timeout,
                model=FinancialApprovalRequestListResponse,
            )
        )

    def revoke_mandate(
        self, mandate_id: str, *, timeout: float | None = None
    ) -> FinancialMandate:
        path = f"/v1/financial/mandates/{quote(mandate_id, safe='')}/revoke"
        return self._send_get_or_post(
            path, method="POST", timeout=timeout, model=FinancialMandate
        )

    def get_receipt(
        self, receipt_id: str, *, timeout: float | None = None
    ) -> FinancialReceipt:
        path = f"/v1/financial/receipts/{quote(receipt_id, safe='')}"
        return self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="GET", timeout=timeout, model=FinancialReceipt
            )
        )

    def get_financial_decision_receipt(
        self, action_id: str, *, timeout: float | None = None
    ) -> FinancialActionDecisionReceipt:
        path = f"/v1/financial/actions/{quote(action_id, safe='')}/decision-receipt"
        return self._run_with_retry(
            lambda: self._send_get_or_post(
                path,
                method="GET",
                timeout=timeout,
                model=FinancialActionDecisionReceipt,
            )
        )

    def record_action_outcome(
        self,
        action_id: str,
        outcome: FinancialActionOutcome,
        *,
        timeout: float | None = None,
    ) -> FinancialActionOutcome:
        path = f"/v1/financial/actions/{quote(action_id, safe='')}/outcomes"
        return self._send_json_model(
            path,
            method="POST",
            body=outcome.model_dump(mode="json", exclude_none=True),
            timeout=timeout,
            model=FinancialActionOutcome,
        )

    def list_action_outcomes(
        self, action_id: str, *, timeout: float | None = None
    ) -> FinancialOutcomeListResponse:
        path = f"/v1/financial/actions/{quote(action_id, safe='')}/outcomes"
        return self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="GET", timeout=timeout, model=FinancialOutcomeListResponse
            )
        )

    def approve_action(
        self, action_id: str, *, timeout: float | None = None
    ) -> FinancialActionRecord:
        return self._transition_financial_action(action_id, "approve", timeout=timeout)

    def deny_action(
        self, action_id: str, *, timeout: float | None = None
    ) -> FinancialActionRecord:
        return self._transition_financial_action(action_id, "deny", timeout=timeout)

    def execute_action(
        self, action_id: str, *, timeout: float | None = None
    ) -> FinancialActionRecord:
        return self._transition_financial_action(action_id, "execute", timeout=timeout)

    def _transition_financial_action(
        self, action_id: str, transition: str, *, timeout: float | None = None
    ) -> FinancialActionRecord:
        path = f"/v1/financial/actions/{quote(action_id, safe='')}/{transition}"
        return self._send_get_or_post(
            path, method="POST", timeout=timeout, model=FinancialActionRecord
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

    def _send_text_model(
        self,
        path: str,
        *,
        method: str,
        body: str,
        content_type: str,
        timeout: float | None,
        model: Any,
    ) -> Any:
        try:
            headers = {**self._headers(), "content-type": content_type}
            resp = self._http.request(
                method,
                path,
                content=body,
                headers=headers,
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


class _AsyncRunContext:
    def __init__(
        self, client: AsyncClient, req: CreateRunRequest, finish_on_error: bool
    ) -> None:
        self._client = client
        self._req = req
        self._finish_on_error = finish_on_error
        self._token: contextvars.Token[dict[str, str]] | None = None
        self.summary: RunSummary | None = None
        self._finished = False

    @property
    def id(self) -> str:
        if self.summary is None:
            raise RuntimeError("run context has not started")
        return self.summary.id

    async def __aenter__(self) -> "_AsyncRunContext":
        self.summary = await self._client.start_run(self._req)
        self._token = _run_context.set(_run_only_context(self.id))
        return self

    async def __aexit__(self, exc_type: Any, *_: Any) -> None:
        try:
            if not self._finished:
                if exc_type is None:
                    await self.finish()
                elif self._finish_on_error:
                    await self.finish(RunStatus.failed)
        finally:
            if self._token is not None:
                _run_context.reset(self._token)

    async def finish(self, status: RunStatus = RunStatus.completed) -> RunSummary:
        summary = await self._client.finish_run(self.id, status)
        self._finished = True
        return summary

    def event(
        self, req: CreateRunEventRequest | None = None, **kwargs: Any
    ) -> "_AsyncRunEventContext":
        return _AsyncRunEventContext(
            self._client, self.id, req or CreateRunEventRequest(**kwargs)
        )


class _AsyncRunEventContext:
    def __init__(
        self, client: AsyncClient, run_id: str, req: CreateRunEventRequest
    ) -> None:
        self._client = client
        self._run_id = run_id
        self._req = req
        self._token: contextvars.Token[dict[str, str]] | None = None
        self.summary: RunEventSummary | None = None

    async def __aenter__(self) -> "_AsyncRunEventContext":
        self.summary = await self._client.create_run_event(self._run_id, self._req)
        self._token = _run_context.set(
            {**_run_context.get(), "run_id": self._run_id, "run_event_id": self.summary.id}
        )
        return self

    async def __aexit__(self, *_: Any) -> None:
        if self._token is not None:
            _run_context.reset(self._token)


class _RunContext:
    def __init__(
        self, client: Client, req: CreateRunRequest, finish_on_error: bool
    ) -> None:
        self._client = client
        self._req = req
        self._finish_on_error = finish_on_error
        self._token: contextvars.Token[dict[str, str]] | None = None
        self.summary: RunSummary | None = None
        self._finished = False

    @property
    def id(self) -> str:
        if self.summary is None:
            raise RuntimeError("run context has not started")
        return self.summary.id

    def __enter__(self) -> "_RunContext":
        self.summary = self._client.start_run(self._req)
        self._token = _run_context.set(_run_only_context(self.id))
        return self

    def __exit__(self, exc_type: Any, *_: Any) -> None:
        try:
            if not self._finished:
                if exc_type is None:
                    self.finish()
                elif self._finish_on_error:
                    self.finish(RunStatus.failed)
        finally:
            if self._token is not None:
                _run_context.reset(self._token)

    def finish(self, status: RunStatus = RunStatus.completed) -> RunSummary:
        summary = self._client.finish_run(self.id, status)
        self._finished = True
        return summary

    def event(
        self, req: CreateRunEventRequest | None = None, **kwargs: Any
    ) -> "_RunEventContext":
        return _RunEventContext(
            self._client, self.id, req or CreateRunEventRequest(**kwargs)
        )


class _RunEventContext:
    def __init__(
        self, client: Client, run_id: str, req: CreateRunEventRequest
    ) -> None:
        self._client = client
        self._run_id = run_id
        self._req = req
        self._token: contextvars.Token[dict[str, str]] | None = None
        self.summary: RunEventSummary | None = None

    def __enter__(self) -> "_RunEventContext":
        self.summary = self._client.create_run_event(self._run_id, self._req)
        self._token = _run_context.set(
            {**_run_context.get(), "run_id": self._run_id, "run_event_id": self.summary.id}
        )
        return self

    def __exit__(self, *_: Any) -> None:
        if self._token is not None:
            _run_context.reset(self._token)


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

    async def submit_event(
        self, event: GuardEvent, *, timeout: float | None = None
    ) -> Decision:
        """Submit a full ``GuardEvent`` (sources + provenance) for a runtime decision."""
        event = _merge_context(event)
        return await self._run_with_retry(
            lambda: self._send_json_model(
                "/v1/events",
                method="POST",
                body=event.model_dump(mode="json", exclude_none=True),
                timeout=timeout,
                model=Decision,
            )
        )

    def run(
        self,
        *,
        agent_id: str,
        kind: RunKind = RunKind.other,
        external_id: str | None = None,
        metadata: dict[str, Any] | None = None,
        input_summary: str | None = None,
        finish_on_error: bool = True,
    ) -> "_AsyncRunContext":
        return _AsyncRunContext(
            self,
            _run_request(
                agent_id=agent_id,
                kind=kind,
                external_id=external_id,
                metadata=metadata,
                input_summary=input_summary,
            ),
            finish_on_error,
        )

    async def guard_tool_call(
        self,
        *,
        agent_id: str,
        operation: str,
        parameters: dict[str, Any] | None = None,
        side_effect: str | SideEffectClass | None = None,
        sources: list[Any] | None = None,
        provenance: dict[str, list[str]] | None = None,
        context: dict[str, Any] | None = None,
    ) -> Decision:
        return await self.submit_event(
            GuardEvent.model_validate(
                {
                    "kind": EventKind.tool_call_proposed,
                    "principal": {
                        "workspace_id": "",
                        "environment_id": "",
                        "agent_id": agent_id,
                    },
                    "action": {
                        "operation": operation,
                        "parameters": parameters or {},
                        **({"side_effect": side_effect} if side_effect else {}),
                    },
                    "sources": sources or [],
                    "provenance": provenance or {},
                    "context": context,
                }
            )
        )

    async def verify_action(
        self, req: CreateFinancialActionRequest, *, timeout: float | None = None
    ) -> FinancialActionRecord:
        """Async variant of ``Client.verify_action``."""
        return await self._run_with_retry(
            lambda: self._send_json_model(
                "/v1/financial/actions",
                method="POST",
                body=req.model_dump(mode="json", exclude_none=True),
                timeout=timeout,
                model=FinancialActionRecord,
            )
        )

    async def guard_payment(
        self, req: CreateFinancialActionRequest, *, timeout: float | None = None
    ) -> FinancialActionRecord:
        """Async alias for payment/refund callers."""
        return await self.verify_action(req, timeout=timeout)

    def financial_operation(
        self,
        *,
        operation: str,
        kind: FinancialActionKind,
        principal_id: str,
        rail: FinancialRail,
        amount: Callable[[InputT, FactsT | None], MoneyAmount],
        idempotency_key: Callable[[InputT, FactsT | None], str],
        counterparty: Callable[[InputT, FactsT | None], CounterpartyRef | None] | None = None,
        mandate: Callable[[InputT, FactsT | None], MandateRef | None] | None = None,
        memo: Callable[[InputT, FactsT | None], str | None] | None = None,
        metadata: Callable[[InputT, FactsT | None], dict[str, Any] | None] | None = None,
        evidence: Callable[[InputT, FactsT | None], list[EvidenceRef]] | None = None,
        execute: bool = False,
    ) -> AsyncFinancialOperation[InputT, FactsT]:
        return AsyncFinancialOperation(
            self,
            operation=operation,
            kind=kind,
            principal_id=principal_id,
            rail=rail,
            amount=amount,
            idempotency_key=idempotency_key,
            counterparty=counterparty,
            mandate=mandate,
            memo=memo,
            metadata=metadata,
            evidence=evidence,
            execute=execute,
        )

    async def get_financial_action(
        self, action_id: str, *, timeout: float | None = None
    ) -> FinancialActionRecord:
        path = f"/v1/financial/actions/{quote(action_id, safe='')}"
        return await self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="GET", timeout=timeout, model=FinancialActionRecord
            )
        )

    async def list_financial_actions(
        self, *, timeout: float | None = None
    ) -> FinancialActionListResponse:
        return await self._run_with_retry(
            lambda: self._send_get_or_post(
                "/v1/financial/actions",
                method="GET",
                timeout=timeout,
                model=FinancialActionListResponse,
            )
        )

    async def authorize_agentic_payment(
        self, req: AgenticPaymentAuthorizeRequest, *, timeout: float | None = None
    ) -> AgenticPaymentAuthorizationResponse:
        return await self._run_with_retry(
            lambda: self._send_json_model(
                "/v1/financial/agentic-payments/authorize",
                method="POST",
                body=req.model_dump(mode="json", exclude_none=True),
                timeout=timeout,
                model=AgenticPaymentAuthorizationResponse,
            )
        )

    async def get_agentic_payment(
        self, action_id: str, *, timeout: float | None = None
    ) -> AgenticPaymentRecord:
        path = f"/v1/financial/agentic-payments/{quote(action_id, safe='')}"
        return await self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="GET", timeout=timeout, model=AgenticPaymentRecord
            )
        )

    async def commit_agentic_payment(
        self,
        action_id: str,
        req: AgenticPaymentCommitRequest,
        *,
        timeout: float | None = None,
    ) -> AgenticPaymentRecord:
        path = f"/v1/financial/agentic-payments/{quote(action_id, safe='')}/commit"
        return await self._send_json_model(
            path,
            method="POST",
            body=req.model_dump(mode="json", exclude_none=True),
            timeout=timeout,
            model=AgenticPaymentRecord,
        )

    async def rollback_agentic_payment(
        self,
        action_id: str,
        req: AgenticPaymentRollbackRequest,
        *,
        timeout: float | None = None,
    ) -> AgenticPaymentRecord:
        path = f"/v1/financial/agentic-payments/{quote(action_id, safe='')}/rollback"
        return await self._send_json_model(
            path,
            method="POST",
            body=req.model_dump(mode="json", exclude_none=True),
            timeout=timeout,
            model=AgenticPaymentRecord,
        )

    async def get_agentic_payment_receipt(
        self, action_id: str, *, timeout: float | None = None
    ) -> FinancialReceipt:
        path = f"/v1/financial/agentic-payments/{quote(action_id, safe='')}/receipt"
        return await self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="GET", timeout=timeout, model=FinancialReceipt
            )
        )

    async def commit_financial_action(
        self,
        action_id: str,
        req: CommitFinancialActionRequest,
        *,
        timeout: float | None = None,
    ) -> CommitFinancialActionResponse:
        path = f"/v1/financial/actions/{quote(action_id, safe='')}/commit"
        return await self._send_json_model(
            path,
            method="POST",
            body=req.model_dump(mode="json", exclude_none=True),
            timeout=timeout,
            model=CommitFinancialActionResponse,
        )

    async def create_financial_execution_connector(
        self,
        req: CreateFinancialExecutionConnectorRequest,
        *,
        timeout: float | None = None,
    ) -> CreateFinancialExecutionConnectorResponse:
        return await self._send_json_model(
            "/v1/financial/execution-connectors",
            method="POST",
            body=req.model_dump(mode="json", exclude_none=True),
            timeout=timeout,
            model=CreateFinancialExecutionConnectorResponse,
        )

    async def list_financial_execution_connectors(
        self, *, timeout: float | None = None
    ) -> FinancialExecutionConnectorListResponse:
        return await self._run_with_retry(
            lambda: self._send_get_or_post(
                "/v1/financial/execution-connectors",
                method="GET",
                timeout=timeout,
                model=FinancialExecutionConnectorListResponse,
            )
        )

    async def revoke_financial_execution_connector(
        self, connector_id: str, *, timeout: float | None = None
    ) -> FinancialExecutionConnector:
        path = (
            f"/v1/financial/execution-connectors/{quote(connector_id, safe='')}/revoke"
        )
        return await self._send_get_or_post(
            path, method="POST", timeout=timeout, model=FinancialExecutionConnector
        )

    async def financial_observation_summary(
        self, start: str, end: str, *, timeout: float | None = None
    ) -> FinancialObservationSummaryResponse:
        path = f"/v1/financial/observations/summary?{urlencode({'start': start, 'end': end})}"
        return await self._run_with_retry(
            lambda: self._send_get_or_post(
                path,
                method="GET",
                timeout=timeout,
                model=FinancialObservationSummaryResponse,
            )
        )

    async def create_financial_observation_review(
        self,
        action_id: str,
        req: CreateFinancialObservationReviewRequest,
        *,
        timeout: float | None = None,
    ) -> FinancialObservationReview:
        path = (
            f"/v1/financial/actions/{quote(action_id, safe='')}/observation-reviews"
        )
        return await self._send_json_model(
            path,
            method="POST",
            body=req.model_dump(mode="json", exclude_none=True),
            timeout=timeout,
            model=FinancialObservationReview,
        )

    async def list_financial_observation_reviews(
        self, action_id: str, *, timeout: float | None = None
    ) -> FinancialObservationReviewListResponse:
        path = (
            f"/v1/financial/actions/{quote(action_id, safe='')}/observation-reviews"
        )
        return await self._run_with_retry(
            lambda: self._send_get_or_post(
                path,
                method="GET",
                timeout=timeout,
                model=FinancialObservationReviewListResponse,
            )
        )

    async def create_financial_policy(
        self, req: CreateFinancialPolicyRequest, *, timeout: float | None = None
    ) -> FinancialPolicyRecord:
        """Async variant of ``Client.create_financial_policy``."""
        return await self._send_json_model(
            "/v1/financial/policies",
            method="POST",
            body=req.model_dump(mode="json", exclude_none=True),
            timeout=timeout,
            model=FinancialPolicyRecord,
        )

    async def list_financial_policies(
        self, *, timeout: float | None = None
    ) -> FinancialPolicyListResponse:
        return await self._run_with_retry(
            lambda: self._send_get_or_post(
                "/v1/financial/policies",
                method="GET",
                timeout=timeout,
                model=FinancialPolicyListResponse,
            )
        )

    async def list_policies(
        self,
        *,
        family: PolicyFamily | None = None,
        timeout: float | None = None,
    ) -> PolicyListResponse:
        path = "/v1/policies"
        if family is not None:
            path = f"{path}?family={quote(family.value, safe='')}"
        return await self._run_with_retry(
            lambda: self._send_get_or_post(
                path,
                method="GET",
                timeout=timeout,
                model=PolicyListResponse,
            )
        )

    async def get_policy(
        self, policy_id: str, *, timeout: float | None = None
    ) -> PolicyDocument:
        path = f"/v1/policies/{quote(policy_id, safe='')}"
        return await self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="GET", timeout=timeout, model=PolicyDocument
            )
        )

    async def upsert_policy(
        self, source_yaml: str, *, timeout: float | None = None
    ) -> PolicyDocument:
        return await self._run_with_retry(
            lambda: self._send_text_model(
                "/v1/policies",
                method="POST",
                body=source_yaml,
                content_type="application/yaml",
                timeout=timeout,
                model=PolicyDocument,
            )
        )

    async def create_mandate(
        self, req: CreateFinancialMandateRequest, *, timeout: float | None = None
    ) -> FinancialMandate:
        return await self._send_json_model(
            "/v1/financial/mandates",
            method="POST",
            body=req.model_dump(mode="json", exclude_none=True),
            timeout=timeout,
            model=FinancialMandate,
        )

    async def list_mandates(
        self, *, timeout: float | None = None
    ) -> FinancialMandateListResponse:
        return await self._run_with_retry(
            lambda: self._send_get_or_post(
                "/v1/financial/mandates",
                method="GET",
                timeout=timeout,
                model=FinancialMandateListResponse,
            )
        )

    async def list_approval_requests(
        self, *, timeout: float | None = None
    ) -> FinancialApprovalRequestListResponse:
        return await self._run_with_retry(
            lambda: self._send_get_or_post(
                "/v1/financial/approval-requests",
                method="GET",
                timeout=timeout,
                model=FinancialApprovalRequestListResponse,
            )
        )

    async def revoke_mandate(
        self, mandate_id: str, *, timeout: float | None = None
    ) -> FinancialMandate:
        path = f"/v1/financial/mandates/{quote(mandate_id, safe='')}/revoke"
        return await self._send_get_or_post(
            path, method="POST", timeout=timeout, model=FinancialMandate
        )

    async def get_receipt(
        self, receipt_id: str, *, timeout: float | None = None
    ) -> FinancialReceipt:
        path = f"/v1/financial/receipts/{quote(receipt_id, safe='')}"
        return await self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="GET", timeout=timeout, model=FinancialReceipt
            )
        )

    async def get_financial_decision_receipt(
        self, action_id: str, *, timeout: float | None = None
    ) -> FinancialActionDecisionReceipt:
        path = f"/v1/financial/actions/{quote(action_id, safe='')}/decision-receipt"
        return await self._run_with_retry(
            lambda: self._send_get_or_post(
                path,
                method="GET",
                timeout=timeout,
                model=FinancialActionDecisionReceipt,
            )
        )

    async def record_action_outcome(
        self,
        action_id: str,
        outcome: FinancialActionOutcome,
        *,
        timeout: float | None = None,
    ) -> FinancialActionOutcome:
        path = f"/v1/financial/actions/{quote(action_id, safe='')}/outcomes"
        return await self._send_json_model(
            path,
            method="POST",
            body=outcome.model_dump(mode="json", exclude_none=True),
            timeout=timeout,
            model=FinancialActionOutcome,
        )

    async def list_action_outcomes(
        self, action_id: str, *, timeout: float | None = None
    ) -> FinancialOutcomeListResponse:
        path = f"/v1/financial/actions/{quote(action_id, safe='')}/outcomes"
        return await self._run_with_retry(
            lambda: self._send_get_or_post(
                path, method="GET", timeout=timeout, model=FinancialOutcomeListResponse
            )
        )

    async def approve_action(
        self, action_id: str, *, timeout: float | None = None
    ) -> FinancialActionRecord:
        return await self._transition_financial_action(action_id, "approve", timeout=timeout)

    async def deny_action(
        self, action_id: str, *, timeout: float | None = None
    ) -> FinancialActionRecord:
        return await self._transition_financial_action(action_id, "deny", timeout=timeout)

    async def execute_action(
        self, action_id: str, *, timeout: float | None = None
    ) -> FinancialActionRecord:
        return await self._transition_financial_action(action_id, "execute", timeout=timeout)

    async def _transition_financial_action(
        self, action_id: str, transition: str, *, timeout: float | None = None
    ) -> FinancialActionRecord:
        path = f"/v1/financial/actions/{quote(action_id, safe='')}/{transition}"
        return await self._send_get_or_post(
            path, method="POST", timeout=timeout, model=FinancialActionRecord
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

    async def _send_text_model(
        self,
        path: str,
        *,
        method: str,
        body: str,
        content_type: str,
        timeout: float | None,
        model: Any,
    ) -> Any:
        try:
            headers = {**self._headers(), "content-type": content_type}
            resp = await self._http.request(
                method,
                path,
                content=body,
                headers=headers,
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

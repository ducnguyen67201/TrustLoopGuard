"""Load first-party LiveKit model selection from the canonical routing manifest."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Optional


_SUPPORTED_SCHEMA_VERSION = 1
_SUPPORTED_REASONING_EFFORTS = {"none", "low", "medium", "high", "xhigh", "max"}
_DEFAULT_MANIFEST_PATH = Path(__file__).resolve().parents[2] / "config" / "llm-routing.json"


@dataclass(frozen=True)
class DemoModelRoute:
    model: str
    reasoning_effort: Optional[str]


def load_demo_model_route(
    route_name: str = "demo_livekit",
    manifest_path: Optional[Path] = None,
) -> DemoModelRoute:
    """Return a validated OpenAI demo route without consulting model env vars."""

    path = manifest_path or _DEFAULT_MANIFEST_PATH
    try:
        manifest = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise RuntimeError(f"Unable to load LLM routing manifest at {path}: {error}") from error

    if not isinstance(manifest, dict):
        raise RuntimeError("LLM routing manifest must be a JSON object")
    if manifest.get("schema_version") != _SUPPORTED_SCHEMA_VERSION:
        raise RuntimeError(
            f"LLM routing manifest must use schema_version {_SUPPORTED_SCHEMA_VERSION}"
        )

    routes = manifest.get("routes")
    if not isinstance(routes, dict):
        raise RuntimeError("LLM routing manifest must define a routes object")
    route = routes.get(route_name)
    if not isinstance(route, dict):
        raise RuntimeError(f'LLM routing manifest is missing route "{route_name}"')
    primary = route.get("primary")
    if not isinstance(primary, dict):
        raise RuntimeError(f'LLM route "{route_name}" must define a primary target')
    if primary.get("provider") != "openai":
        raise RuntimeError(f'LLM route "{route_name}" must use the openai provider')

    model = primary.get("model")
    if not isinstance(model, str) or not model.strip():
        raise RuntimeError(f'LLM route "{route_name}" must define a non-empty model')

    reasoning_effort = primary.get("reasoning_effort")
    if reasoning_effort is not None and (
        not isinstance(reasoning_effort, str)
        or reasoning_effort not in _SUPPORTED_REASONING_EFFORTS
    ):
        raise RuntimeError(
            f'LLM route "{route_name}" has unsupported reasoning_effort '
            f'"{reasoning_effort}"'
        )

    return DemoModelRoute(model=model.strip(), reasoning_effort=reasoning_effort)

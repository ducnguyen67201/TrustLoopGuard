import json
import tempfile
import unittest
from pathlib import Path

from demo.livekit.model_routing import load_demo_model_route


class ModelRoutingTest(unittest.TestCase):
    def test_loads_committed_livekit_route(self) -> None:
        route = load_demo_model_route()

        self.assertEqual(route.model, "gpt-4o-mini")
        self.assertIsNone(route.reasoning_effort)

    def test_rejects_unsupported_reasoning_effort(self) -> None:
        fixture = {
            "schema_version": 1,
            "providers": {
                "openai": {"kind": "openai", "api_key_env": "OPENAI_API_KEY"}
            },
            "routes": {
                "demo_livekit": {
                    "primary": {
                        "provider": "openai",
                        "model": "gpt-4o-mini",
                        "deadline_ms": 30000,
                        "reasoning_effort": "fast",
                    }
                }
            },
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "llm-routing.json"
            path.write_text(json.dumps(fixture), encoding="utf-8")

            with self.assertRaisesRegex(RuntimeError, "unsupported reasoning_effort"):
                load_demo_model_route(manifest_path=path)

    def test_rejects_missing_route(self) -> None:
        fixture = {"schema_version": 1, "routes": {}}
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "llm-routing.json"
            path.write_text(json.dumps(fixture), encoding="utf-8")

            with self.assertRaisesRegex(RuntimeError, "missing route"):
                load_demo_model_route(manifest_path=path)

    def test_resolves_openai_provider_alias(self) -> None:
        fixture = {
            "schema_version": 1,
            "providers": {
                "first_party": {"kind": "openai", "api_key_env": "OPENAI_API_KEY"}
            },
            "routes": {
                "demo_livekit": {
                    "primary": {
                        "provider": "first_party",
                        "model": "gpt-4o-mini",
                        "deadline_ms": 30000,
                    }
                }
            },
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "llm-routing.json"
            path.write_text(json.dumps(fixture), encoding="utf-8")

            route = load_demo_model_route(manifest_path=path)

        self.assertEqual(route.model, "gpt-4o-mini")

    def test_rejects_provider_identifier_with_non_openai_kind(self) -> None:
        fixture = {
            "schema_version": 1,
            "providers": {
                "openai": {"kind": "openrouter", "api_key_env": "OPENROUTER_API_KEY"}
            },
            "routes": {
                "demo_livekit": {
                    "primary": {
                        "provider": "openai",
                        "model": "openai/gpt-4o-mini",
                        "deadline_ms": 30000,
                    }
                }
            },
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "llm-routing.json"
            path.write_text(json.dumps(fixture), encoding="utf-8")

            with self.assertRaisesRegex(RuntimeError, "must use an openai provider"):
                load_demo_model_route(manifest_path=path)


if __name__ == "__main__":
    unittest.main()

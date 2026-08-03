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


if __name__ == "__main__":
    unittest.main()

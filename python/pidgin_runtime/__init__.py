"""Pidgin Python SDK — subprocess wrapper around the pgn binary."""

import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Optional


class PidginClient:
    """Client that shells out to the pgn binary."""

    def __init__(self, binary: str = "pgn") -> None:
        self._binary = binary

    def _run(self, args: list[str]) -> str:
        result = subprocess.run(
            [self._binary, *args],
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            raise RuntimeError(result.stderr.strip())
        return result.stdout

    def parse(self, file: Path) -> dict[str, Any]:
        output = self._run(["parse", str(file)])
        return self._parse_debug_output(output)

    def validate(self, file: Path, host: Path = Path(".")) -> bool:
        try:
            self._run(["validate", str(file), "--host", str(host)])
            return True
        except RuntimeError:
            return False

    def check(self, file: Path, host: Path = Path(".")) -> dict[str, Any]:
        output = self._run(["check", str(file), "--host", str(host)])
        return {"output": output.strip()}

    def measure(self, file: Path) -> dict[str, Any]:
        output = self._run(["measure", str(file)])
        return self._parse_yaml_lines(output)

    def expand(self, file: Path, host: Path = Path(".")) -> dict[str, Any]:
        output = self._run(["expand", str(file), "--host", str(host)])
        return self._parse_yaml_lines(output)

    def resolve(self, file: Path, host: Path = Path(".")) -> list[dict[str, Any]]:
        output = self._run(["resolve", str(file), "--host", str(host)])
        lines = output.strip().split("\n")[1:]
        results = []
        for line in lines:
            parts = line.strip().split()
            if len(parts) >= 6:
                results.append({
                    "status": parts[0],
                    "ns": parts[1].split("=")[1],
                    "id": parts[2].split("=")[1],
                    "confidence": float(parts[3].split("=")[1]),
                    "required": parts[4].split("=")[1] == "true",
                    "path": parts[5].split("=")[1] if len(parts) > 5 else "",
                })
        return results

    @staticmethod
    def _parse_debug_output(output: str) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for line in output.split("\n"):
            if ":" in line:
                key, _, value = line.partition(":")
                result[key.strip()] = value.strip()
        return result

    @staticmethod
    def _parse_yaml_lines(output: str) -> dict[str, Any]:
        import yaml
        return yaml.safe_load(output)


def main() -> None:
    """Simple CLI for testing the Python wrapper."""
    if len(sys.argv) < 2:
        print("Usage: pgn-py <command> <file> [--host .]")
        sys.exit(1)

    command = sys.argv[1]
    file = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(".")
    host = Path(".")
    if "--host" in sys.argv:
        idx = sys.argv.index("--host")
        if idx + 1 < len(sys.argv):
            host = Path(sys.argv[idx + 1])

    client = PidginClient("target/debug/pidgin-cli")

    if command == "parse":
        result = client.parse(file)
        print(json.dumps(result, indent=2))
    elif command == "validate":
        ok = client.validate(file, host)
        print("PASS" if ok else "FAIL")
        sys.exit(0 if ok else 1)
    elif command == "measure":
        result = client.measure(file)
        print(json.dumps(result, indent=2))
    elif command == "expand":
        result = client.expand(file, host)
        print(json.dumps(result, indent=2))
    elif command == "resolve":
        results = client.resolve(file, host)
        print(json.dumps(results, indent=2))
    else:
        print(f"Unknown command: {command}")
        sys.exit(1)


if __name__ == "__main__":
    main()

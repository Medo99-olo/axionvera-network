#!/usr/bin/env python3
import json
import sys
from pathlib import Path

required = [
    "network",
    "rpcUrl",
    "vaultContractId",
    "depositTokenContractId",
    "rewardTokenContractId",
]

path = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("examples/sdk-handoff.json")
data = json.loads(path.read_text())

missing = [key for key in required if not data.get(key)]
if missing:
    raise SystemExit(f"Missing required fields: {', '.join(missing)}")

if not data["rpcUrl"].startswith(("http://", "https://")):
    raise SystemExit("rpcUrl must start with http:// or https://")

print(f"SDK handoff valid: {path}")

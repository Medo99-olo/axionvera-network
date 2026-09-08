#!/usr/bin/env python3
import json
import sys
from pathlib import Path

required = ["network", "vaultContractId", "wasmHash", "deployerAddress", "deployedAt"]

path = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("examples/contract-id-registry.json")
data = json.loads(path.read_text())

missing = [key for key in required if not data.get(key)]
if missing:
    raise SystemExit(f"Missing required fields: {', '.join(missing)}")

if data["network"] not in ["local", "testnet", "mainnet", "futurenet"]:
    raise SystemExit(f"Unsupported network: {data['network']}")

print(f"Contract ID registry valid: {path}")

"""Check base catalog parity and coverage of literal UI translation calls."""
import json
import pathlib
import re

root = pathlib.Path(__file__).resolve().parent.parent
catalogs = {code: json.loads((root / "locales" / f"{code}.json").read_text(encoding="utf-8")) for code in ("fr", "en")}
assert catalogs["fr"].keys() == catalogs["en"].keys(), "FR/EN key mismatch"
keys = set()
for path in (root / "src" / "ui").glob("*.rs"):
    if path.name.endswith("tests.rs"):
        continue
    for value in re.findall(r'(?:crate::i18n::tr|\btr)\(\s*("(?:[^"\\]|\\.)*")', path.read_text(encoding="utf-8")):
        keys.add(json.loads(value))
for code, catalog in catalogs.items():
    assert not keys - catalog.keys(), f"Missing {code} keys: {keys - catalog.keys()}"
    assert all(isinstance(value, str) and value for value in catalog.values()), f"Invalid {code} value"
print(f"FR/EN: {len(catalogs['en'])} keys; all literal UI translation calls covered.")

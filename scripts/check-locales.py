"""Check base catalog parity and coverage of literal UI translation calls."""
import json
import pathlib
import re
from collections import Counter

root = pathlib.Path(__file__).resolve().parent.parent
def unique_object(pairs):
    result = {}
    for key, value in pairs:
        assert key not in result, f"Duplicate translation key: {key}"
        result[key] = value
    return result

catalogs = {path.stem: json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_object)
            for path in (root / "locales").glob("*.json")}
expected = {"fr", "en", "es", "it", "de", "pt", "ja", "zh-CN", "ko", "ar", "hi", "id"}
assert expected <= catalogs.keys(), "Missing bundled language"
keys = set()
for path in (root / "src" / "ui").glob("*.rs"):
    if path.name.endswith("tests.rs"):
        continue
    for value in re.findall(r'(?:crate::i18n::tr|\btr)\(\s*("(?:[^"\\]|\\.)*")', path.read_text(encoding="utf-8")):
        keys.add(json.loads(value))
for code, catalog in catalogs.items():
    assert catalog.keys() == catalogs["en"].keys(), f"{code}: catalog key mismatch"
    assert not keys - catalog.keys(), f"Missing {code} keys: {keys - catalog.keys()}"
    assert all(isinstance(value, str) and value for value in catalog.values()), f"Invalid {code} value"
    for key, value in catalog.items():
        assert Counter(re.findall(r"\{[^{}]*\}", key)) == Counter(re.findall(r"\{[^{}]*\}", value)), f"{code}: changed placeholders: {key}"
        assert not re.search(r"ZXPH\d|ZX\d{3}XZ", value), f"{code}: translation marker left"
print(f"{len(catalogs)} languages, {len(catalogs['en'])} keys each; UI coverage and placeholders verified.")

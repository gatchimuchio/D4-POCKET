from pathlib import Path
import copy
import json


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SPECS_DIR = ROOT / "specs"


class SchemaCatalog:
    def __init__(self, specs_dir: Path = DEFAULT_SPECS_DIR):
        self.specs_dir = Path(specs_dir)
        self._schemas: dict[str, dict] = {}

    def load(self) -> "SchemaCatalog":
        schemas: dict[str, dict] = {}
        for path in sorted(self.specs_dir.glob("*.schema.json")):
            schemas[path.name] = json.loads(path.read_text(encoding="utf-8"))
        self._schemas = schemas
        return self

    def names(self) -> tuple[str, ...]:
        return tuple(sorted(self._schemas))

    def get(self, name: str) -> dict:
        if name not in self._schemas:
            raise KeyError(f"読込み済みでない schema です: {name}")
        return copy.deepcopy(self._schemas[name])

    def require(self, names: set[str]) -> None:
        missing = sorted(names - set(self._schemas))
        if missing:
            raise KeyError(f"欠落した schema があります: {', '.join(missing)}")


def load_default_catalog() -> SchemaCatalog:
    return SchemaCatalog().load()

"""Schema を根拠とする GUI Shell 契約の読込み境界。"""

from .schema_loader import SchemaCatalog, load_default_catalog

__all__ = ["SchemaCatalog", "load_default_catalog"]

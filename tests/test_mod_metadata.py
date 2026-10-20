"""Regression tests: python3 -m unittest discover -s tests -v."""

import importlib.util
from pathlib import Path
import tempfile
import unittest
import zipfile


spec = importlib.util.spec_from_file_location(
    "installer", Path(__file__).resolve().parents[1] / "src/install/install.py"
)
installer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(installer)


class ModMetadataTests(unittest.TestCase):
    def read(self, dependencies, metadata="META-INF/mods.toml"):
        with tempfile.TemporaryDirectory() as directory:
            jar = Path(directory) / "mod.jar"
            with zipfile.ZipFile(jar, "w") as archive:
                archive.writestr(metadata, '[[mods]]\nmodId="example"\n' + dependencies)
            return installer.read_mod_metadata(jar)

    def test_flat_and_grouped_dependencies_preserve_required_server_mods(self):
        for metadata in ("META-INF/mods.toml", "META-INF/neoforge.mods.toml"):
            for table in ("dependencies", "dependencies.example"):
                with self.subTest(metadata=metadata, table=table):
                    entries = [
                        'modId="ae2"\nmandatory=true\nside="BOTH"',
                        'modId="ars_nouveau"\nmandatory=true\nside="SERVER"',
                        'modId="optional_mod"\nmandatory=false',
                        'modId="client_mod"\nmandatory=true\nside="CLIENT"',
                        'modId="neo_required"\ntype="required"',
                    ]
                    data = "\n".join(f"[[{table}]]\n{entry}\n" for entry in entries)
                    self.assertEqual(
                        self.read(data, metadata),
                        ({"example"}, False, {"ae2", "ars_nouveau", "neo_required"}),
                    )

    def test_no_dependencies(self):
        self.assertEqual(self.read(""), ({"example"}, False, set()))

    def test_cleanup_keeps_mods_required_by_flat_dependencies(self):
        with tempfile.TemporaryDirectory() as directory:
            mods = Path(directory) / "mods"
            mods.mkdir()
            with zipfile.ZipFile(mods / "parent.jar", "w") as archive:
                archive.writestr(
                    "META-INF/mods.toml",
                    '[[mods]]\nmodId="parent"\n'
                    '[[dependencies]]\nmodId="needed"\nmandatory=true\nside="BOTH"\n',
                )
            for name in ("needed", "unneeded"):
                with zipfile.ZipFile(mods / f"{name}.jar", "w") as archive:
                    archive.writestr(
                        "fabric.mod.json",
                        '{"id":"' + name + '","environment":"client"}',
                    )
            previous_root, previous_excludes = installer.ROOT, installer.EXCLUDE_PATTERNS
            try:
                installer.ROOT = Path(directory)
                installer.EXCLUDE_PATTERNS = []
                self.assertEqual(installer.remove_client_only_mods(), ["unneeded.jar"])
                self.assertTrue((mods / "needed.jar").exists())
                self.assertTrue((mods / "parent.jar").exists())
            finally:
                installer.ROOT, installer.EXCLUDE_PATTERNS = previous_root, previous_excludes


if __name__ == "__main__":
    unittest.main()

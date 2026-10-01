#!/usr/bin/env python3
"""Validate official fixtures and library-generated XML entirely offline.

Requires Python lxml. Exit 1 means a fixture mismatch, library parse/write
failure, malformed output, or output rejected by its XSD.
"""
import argparse
from collections import Counter
import json
from pathlib import Path
import sys
from lxml import etree


class LocalSchemas(etree.Resolver):
    def __init__(self, directory):
        super().__init__()
        self.directory = directory

    def resolve(self, url, public_id, context):
        name = url.rsplit("/", 1)[-1]
        # The BOM XSD imports this namespace URL, not a .xsd filename.
        if name == "spdx":
            name = "spdx.xsd"
        path = self.directory / name
        if not path.is_file():
            raise OSError(f"No local schema for {url}: {path}")
        return self.resolve_filename(str(path), context)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spec-dir", type=Path, default=Path("/home/shnatsel/Code/cyclonedx-specification"))
    parser.add_argument("--versions", nargs="+", default=["1.3", "1.4", "1.5", "1.6", "1.7"])
    parser.add_argument("--fixtures", action="store_true", help="Check valid*.xml and invalid*.xml against their expected outcomes")
    parser.add_argument("--outputs", type=Path, help="Check XML output and parse failures recorded in the probe's manifest.json")
    parser.add_argument("files", nargs="*", type=Path, help="Additional XML documents to validate; version comes from the root namespace")
    args = parser.parse_args()
    if not (args.fixtures or args.outputs or args.files):
        parser.error("Specify --fixtures, --outputs, or XML filenames")

    schema_parser = etree.XMLParser(no_network=True, resolve_entities=False)
    schema_parser.resolvers.add(LocalSchemas(args.spec_dir / "schema"))
    schemas = {}

    def schema(version):
        if version not in schemas:
            path = args.spec_dir / "schema" / f"bom-{version}.xsd"
            schemas[version] = etree.XMLSchema(etree.parse(str(path), schema_parser))
        return schemas[version]

    def check(path, version=None):
        try:
            document = etree.parse(str(path), etree.XMLParser(no_network=True, resolve_entities=False))
        except etree.XMLSyntaxError as error:
            return False, str(error)
        if version is None:
            namespace = etree.QName(document.getroot()).namespace or ""
            prefix = "http://cyclonedx.org/schema/bom/"
            if not namespace.startswith(prefix):
                return False, f"Unrecognized BOM namespace: {namespace!r}"
            version = namespace[len(prefix):]
        validator = schema(version)
        valid = validator.validate(document)
        return valid, " | ".join(str(error) for error in list(validator.error_log)[:3])

    failures = 0
    if args.fixtures:
        for version in args.versions:
            directory = args.spec_dir / "tools/src/test/resources" / version
            counts = Counter()
            paths = sorted(directory.glob("valid*.xml")) + sorted(directory.glob("invalid*.xml"))
            if not paths:
                raise FileNotFoundError(f"No XML fixtures in {directory}")
            for path in paths:
                expected = path.name.startswith("valid")
                valid, error = check(path, version)
                counts["valid" if expected else "invalid"] += 1
                if valid != expected:
                    failures += 1
                    counts["mismatch"] += 1
                    print(f"FIXTURE MISMATCH {path}: expected {'valid' if expected else 'invalid'}; {error}")
            print(f"FIXTURES {version}: {counts['valid']} valid, {counts['invalid']} invalid; {counts['mismatch']} mismatches")

    if args.outputs:
        records = json.loads((args.outputs / "manifest.json").read_text())
        totals = {}
        for record in records:
            version = record["version"]
            if version not in args.versions:
                continue
            counts = totals.setdefault(version, Counter())
            counts["inputs"] += 1
            failure = record.get("parse_error") or record.get("xml_error")
            if failure:
                counts["library_errors"] += 1
                failures += 1
                print(f"LIBRARY ERROR {record['input']}: {failure}")
                continue
            if "xml" not in record:
                raise ValueError(f"Missing XML result: {record}")
            path = args.outputs / record["xml"]
            valid, error = check(path, version)
            counts["xml_checked"] += 1
            if not valid:
                counts["xml_failures"] += 1
                failures += 1
                print(f"OUTPUT INVALID {path}: {error}")
        for version, counts in sorted(totals.items()):
            print(f"OUTPUTS {version}: {counts['inputs']} inputs; {counts['library_errors']} library errors; {counts['xml_failures']}/{counts['xml_checked']} XML outputs invalid")

    for path in args.files:
        valid, error = check(path)
        print(f"{'VALID' if valid else 'INVALID'} {path}" + (f": {error}" if error else ""))
        failures += not valid
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())

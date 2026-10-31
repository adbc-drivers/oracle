# Copyright (c) 2025-2026 Columnar Technologies Inc.  All rights reserved.

import argparse
from pathlib import Path

from adbc_drivers_validation import generate_documentation

from . import oracle

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    template = Path(__file__).parent.parent.parent / "docs/oracle.md"
    template = template.resolve()

    reports = [report.resolve() for report in Path(".").glob("validation-report*.xml")]
    generate_documentation.generate(
        "oracle",
        lambda version, vendor: oracle.get_quirks(version),
        [
            ("oracle", "Oracle Database"),
        ],
        reports,
        template,
        args.output.resolve(),
    )

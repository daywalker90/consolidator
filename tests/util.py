import string
import random
import logging
import os
import pytest
from pathlib import Path

RUST_PROFILE = os.environ.get("RUST_PROFILE", "debug")
COMPILED_PATH = Path.cwd() / "target" / RUST_PROFILE / \
    "consolidator"
DOWNLOAD_PATH = Path.cwd() / "tests" / "consolidator"


@pytest.fixture
def get_plugin(directory):
    if COMPILED_PATH.is_file():
        return COMPILED_PATH
    elif DOWNLOAD_PATH.is_file():
        return DOWNLOAD_PATH
    else:
        raise ValueError("No files were found.")

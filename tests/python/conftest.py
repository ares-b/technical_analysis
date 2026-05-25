import importlib

import pytest


@pytest.fixture(scope="session")
def ta():
    return importlib.import_module("technical_analysis")

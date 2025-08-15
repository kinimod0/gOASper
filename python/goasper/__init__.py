from ._lowlevel import PyLayout as _PyLayout


class Layout:
    def __init__(self):
        self._inner = _PyLayout()

    def load_gds(self, path):
        self._inner.load_gds(str(path))

    def save_oas(self, path):
        self._inner.save_oas(str(path))

    def cells(self):
        """Return list of cell (structure) names parsed from the GDS."""
        return self._inner.cell_names()

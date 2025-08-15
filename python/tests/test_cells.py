from goasper import Layout


def test_cells_smoke(tmp_path):
    # This expects you to put a tiny.gds under testdata/
    layout = Layout()
    layout.load_gds("examples/nand2.gds2")
    assert all(
        cell == expected_cell
        for cell, expected_cell in zip(layout.cells(), ["via", "nand2", "inv1", "abc2"])
    )

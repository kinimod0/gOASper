from goasper import Layout

def test_smoke():
    layout = Layout()
    layout.load_gds("examples/nand2.gds2")

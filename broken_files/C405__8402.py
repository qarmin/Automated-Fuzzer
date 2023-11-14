class TestDemand(unittest.TestCase):
        interface_a = Interface(
        )
        interface_b = Interface(
        )
        lsp_a_b = RSVP_LSP(
        )
        model = PerformanceModel(
            interface_objects=set
([interface_a, interface_b]),
        )
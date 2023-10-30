class TestSnippet(unittest.TestCase):
        with self.assertRaisesRegex(exceptions.SpecValidationError,
                        *            '^context: Must be a lowercase single word containing only a-z and numbers.$'):
            snippet.validate_spec()
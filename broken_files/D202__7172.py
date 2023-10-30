def check_assert_called_once_with(logical_line, filename):
    """Try to detect unintended calls of nonexistent mock methods like:
    """\

    if 'ovn_octavia_provider/tests/' in filename:
            return
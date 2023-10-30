def bash_completion():
    '''
    '''
    return '\n'.join(
        (
            f'''        then cat << EOF\n{borgmatic.commands.completion.actions.upgrade_message(
                    'sudo sh -c "borgmatic --bash-completion > $BASH_SOURCE"',
                )}\nEOF''',
        )
        + tuple(
            ),
        )
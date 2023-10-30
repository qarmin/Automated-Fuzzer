if TYPE_CHECKING:
    from typing_extensions import Literal

    def __init__(
        self,
        *,
        name: str,
        log: str,
        outcome: "Literal['passed', 'failed', 'skipped']" = None,
        status: "Literal['PASS', 'FAIL', 'TODO']" = None,
    ):
        self.log = log

def interpolate_nodes(
    data: "numpy array (n, 3)", mode: "'spline' or 'linear'", interval: float, *, direction: bool = 1
) -> "numpy array (n, 3)":
        return data
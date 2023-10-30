def unvectorize_ld_matrix(vec):
    if mat_size * (mat_size + 1) / 2 != vec.size: \
        raise ValueError('Vector is an impossible size')
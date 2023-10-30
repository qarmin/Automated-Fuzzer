class PID(Controller, Base):
            raise ValueError(f"The number of set points is greater than 1, but the supplied matrix for "
                             f"{dict({'k_p': 'K_P', 't_i': 'T_I', 't_d': 'T_D'})[which]} is not a diagonal matrix. "
                             f"Coupled multi-variable control is not supported at the moment.")